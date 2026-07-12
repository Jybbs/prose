import { watchDebounced }   from '@vueuse/core'
import { parse, stringify } from 'smol-toml'
import { computed, ref, type Ref, type WritableComputedRef } from 'vue'

import type { Facet, LengthKnob, RuleControl, SandboxSchema } from '../sandbox/config-schema.data'
import type { LintFinding }                                   from '../fixtures/lint-findings'
import type { SandboxCase }                                   from '../sandbox/pool.data'
import { errorMessage }                                       from '../shared/error-message'
import { loadModule, type ProseWasm }                         from '../sandbox/load-module'

type FacetValue    = boolean | number | string | readonly string[]
type ParsedRule    = boolean | Record<string, FacetValue>
type ParsedConfig  = { rules?: Record<string, ParsedRule> } &
  Record<string, number | Record<string, ParsedRule> | undefined>
type ProbeBaseline = { diagnostics: string, formatted: string }
type SourceProbe   = {
  eligible     : readonly string[]
  facetImpact  : Record<string, readonly string[]>
  lengthImpact : readonly string[]
}

const HASH_PREFIX   = '#1.'
const INT_PROBES    = [1, 500] as const
const LENGTH_PROBES = [30, 180] as const
const STORAGE_KEY   = 'prose-sandbox'

type SharedState = { case?: string, configToml: string, source?: string }

// Deflates the session into a URL-safe hash payload so any sandbox moment
// can travel as a link, returning `null` where the platform lacks the codec.
// An untouched pool example travels as its case id rather than its source,
// keeping the common config-experiment link short.
async function encodeShare(state: SharedState): Promise<string | null> {
  if (typeof CompressionStream === 'undefined') return null
  const bytes  = new TextEncoder().encode(JSON.stringify(state))
  const stream = new Blob([bytes]).stream().pipeThrough(new CompressionStream('deflate-raw'))
  const packed = new Uint8Array(await new Response(stream).arrayBuffer())
  let binary = ''
  for (const byte of packed) binary += String.fromCodePoint(byte)
  return btoa(binary).replaceAll('+', '-').replaceAll('/', '_').replace(/=+$/, '')
}

async function decodeShare(hash: string): Promise<SharedState | null> {
  if (!hash.startsWith(HASH_PREFIX) || typeof DecompressionStream === 'undefined') return null
  try {
    const b64    = hash.slice(HASH_PREFIX.length).replaceAll('-', '+').replaceAll('_', '/')
    const bytes  = Uint8Array.from(atob(b64), char => char.codePointAt(0) ?? 0)
    const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream('deflate-raw'))
    const state  = JSON.parse(await new Response(stream).text()) as SharedState
    const seeded = typeof state.source === 'string' || typeof state.case === 'string'
    return seeded && typeof state.configToml === 'string' ? state : null
  } catch {
    return null
  }
}

export interface ProseSandbox {
  activeCase     : Ref<SandboxCase>
  caseCount      : number
  codeLineLength : WritableComputedRef<number>
  configError    : Ref<string>
  configToml     : Ref<string>
  diagnostics    : Ref<readonly LintFinding[]>
  eligible       : Ref<readonly string[] | null>
  error          : Ref<string>
  facetImpact    : Ref<Record<string, readonly string[]>>
  facetValue     : (slug: string, facet: Facet) => FacetValue
  lengthImpact   : Ref<readonly string[] | null>
  formatted      : Ref<string>
  isMoved        : (slug: string) => boolean
  lengthValue    : (key: string) => number
  lengths        : readonly LengthKnob[]
  refresh        : () => void
  rules          : readonly RuleControl[]
  setFacet       : (slug: string, facet: Facet, value: FacetValue) => void
  setLength      : (key: string, value: number) => void
  share          : () => Promise<string | null>
  source         : Ref<string>
  start          : () => Promise<void>
  status         : Ref<SandboxStatus>
}

export interface ProseSandboxOptions {
  cases       : readonly SandboxCase[]
  schema      : SandboxSchema
  debounceMs ?: number
  load       ?: (reinit: number) => Promise<ProseWasm>
  pick       ?: (count: number, exclude: number) => number
}

type SandboxStatus = 'idle' | 'loading'

const TRAP_NOTICE =
  'The formatter hit an internal error on this input. Edit the source to try again.'

function randomOther(count: number, exclude: number): number {
  if (count <= 1) return 0
  const roll = Math.floor(Math.random() * (count - 1))
  return roll >= exclude ? roll + 1 : roll
}

// A plain deep copy that also unwraps the reactive proxy, which
// `structuredClone` rejects.
function clone(config: ParsedConfig): ParsedConfig {
  return JSON.parse(JSON.stringify(config)) as ParsedConfig
}

// The probe values that could reveal a facet's effect on a source: a bool
// flips its default and an int takes each extreme, whereas a string kind has
// no finite probe set, so `null` marks it unprobeable.
function facetProbes(facet: Facet): readonly FacetValue[] | null {
  if (facet.kind === 'bool') return [facet.default !== true]
  if (facet.kind === 'int') return INT_PROBES
  return null
}

// A facet has impact when any probe run differs from the default-run baseline
// in output or findings. An unprobeable facet or a failed probe proves
// nothing, so both count as impact, leaving the facet visible.
function facetHasImpact(
  baseline : ProbeBaseline,
  facet    : Facet,
  format   : ProseWasm['format'],
  slug     : string,
  source   : string
): boolean {
  const probes = facetProbes(facet)
  if (!probes) return true
  return probes.some(value => {
    try {
      const run = format(stringify({ rules: { [slug]: { [facet.key]: value } } }), source)
      return run.formatted !== baseline.formatted || run.diagnostics !== baseline.diagnostics
    } catch {
      return true
    }
  })
}

// A length knob has impact when either extreme changes the output against
// the default-run baseline. A failed run proves nothing, so it counts as
// impact, keeping the knob visible.
function lengthHasImpact(
  baseline : ProbeBaseline,
  format   : ProseWasm['format'],
  key      : string,
  source   : string
): boolean {
  return LENGTH_PROBES.some(value => {
    try {
      const run = format(stringify({ [key]: value }), source)
      return run.formatted !== baseline.formatted || run.diagnostics !== baseline.diagnostics
    } catch {
      return true
    }
  })
}

const nextMacrotask = (): Promise<void> => new Promise(resolve => { setTimeout(resolve) })

export function useProseSandbox(options: ProseSandboxOptions): ProseSandbox {
  const { cases, schema, debounceMs = 250, load = loadModule, pick = randomOther } = options

  const activeIndex  = ref(0)
  const activeCase   = computed(() => cases[activeIndex.value])
  const configError  = ref('')
  const configToml   = ref('')
  const diagnostics  = ref<readonly LintFinding[]>([])
  const eligible     = ref<readonly string[] | null>(null)
  const facetImpact  = ref<Record<string, readonly string[]>>({})
  const lengthImpact = ref<readonly string[] | null>(null)
  const error        = ref('')
  const formatted    = ref('')
  const parsed       = ref<ParsedConfig>({})
  const source       = ref(cases[0].source)
  const status       = ref<SandboxStatus>('idle')

  let module: ProseWasm | null = null
  let reinit = 0
  let eligibleSource = '\0'

  const probed = new Map<string, SourceProbe>()

  // Probes a source in the background: the default run seeds the eligible set
  // before the first await, then each eligible rule's sub-facets are probed
  // against that baseline one rule per macrotask, so the burst of probe runs
  // never blocks a display format. A source change mid-loop abandons the
  // stale loop, and a finished map is cached so a revisited source replays
  // without any probe runs.
  async function probeSource(current: ProseWasm, target: string): Promise<void> {
    let baseline: ProbeBaseline
    let fired: readonly string[]
    try {
      const run = current.format('', target)
      baseline  = run
      fired     = JSON.parse(run.fired_rules) as string[]
    } catch {
      eligible.value = []
      return
    }
    eligible.value = fired
    await nextMacrotask()
    if (eligibleSource !== target) return
    const lengths = schema.lengths
      .filter(knob => lengthHasImpact(baseline, current.format, knob.key, target))
      .map(knob => knob.key)
    lengthImpact.value = lengths
    const impact: Record<string, readonly string[]> = {}
    for (const rule of schema.rules) {
      if (!fired.includes(rule.slug)) continue
      await nextMacrotask()
      if (eligibleSource !== target) return
      impact[rule.slug] = rule.facets
        .filter(facet => facet.key !== 'enabled')
        .filter(facet => facetHasImpact(baseline, facet, current.format, rule.slug, target))
        .map(facet => facet.key)
      facetImpact.value = { ...impact }
    }
    if (module === current) {
      probed.set(target, { eligible: fired, facetImpact: impact, lengthImpact: lengths })
    }
  }

  function commit(next: ParsedConfig): void {
    if (next.rules && Object.keys(next.rules).length === 0) delete next.rules
    const text        = stringify(next)
    parsed.value      = next
    configToml.value  = text.trim() ? text : ''
    configError.value = ''
  }

  function facetValue(slug: string, facet: Facet): FacetValue {
    const rule = parsed.value.rules?.[slug]
    if (facet.key === 'enabled') {
      if (rule === false) return false
      return typeof rule === 'object' ? rule.enabled ?? true : true
    }
    return typeof rule === 'object' ? rule[facet.key] ?? facet.default : facet.default
  }

  // A rule is moved when it carries any override, whether a disable or a
  // changed facet, so the panel can mark it apart from a resting default.
  function isMoved(slug: string): boolean {
    return parsed.value.rules?.[slug] !== undefined
  }

  function setFacet(slug: string, facet: Facet, value: FacetValue): void {
    const next = clone(parsed.value)
    const rules = (next.rules ??= {})
    const current = rules[slug]
    if (facet.key === 'enabled') {
      if (value === false) rules[slug] = false
      else if (current === false) delete rules[slug]
      else if (typeof current === 'object') delete current.enabled
      commit(next)
      return
    }
    const table = typeof current === 'object' ? current : {}
    if (value === facet.default) delete table[facet.key]
    else table[facet.key] = value
    if (Object.keys(table).length === 0) delete rules[slug]
    else rules[slug] = table
    commit(next)
  }

  function defaultLength(key: string): number {
    return schema.lengths.find(knob => knob.key === key)?.default ?? schema.codeLineLength
  }

  function lengthValue(key: string): number {
    const value = parsed.value[key]
    return typeof value === 'number' ? value : defaultLength(key)
  }

  function setLength(key: string, value: number): void {
    const next = clone(parsed.value)
    if (value === defaultLength(key)) delete next[key]
    else next[key] = value
    commit(next)
  }

  const codeLineLength = computed<number>({
    get : () => lengthValue('code-line-length'),
    set : value => setLength('code-line-length', value)
  })

  async function instantiate(): Promise<ProseWasm> {
    const next = await load(reinit)
    await next.default()
    return next
  }

  async function format(): Promise<void> {
    if (!module) status.value = 'loading'
    try {
      module ??= await instantiate()
      const result = module.format(configToml.value, source.value)
      formatted.value   = result.formatted
      diagnostics.value = result.diagnostics ? JSON.parse(result.diagnostics) : []
      error.value       = ''
      // The rules that fire under the default config are the ones that can
      // affect this snippet, and each eligible rule's sub-facets are probed
      // against that default-run baseline so the panel can hide the knobs
      // that cannot affect it. Recompute only when the source changes, since
      // a config toggle never changes what is eligible. The display already
      // succeeded, so a fault in the probe runs must not reset the module or
      // surface an error over a good format.
      if (source.value !== eligibleSource) {
        eligibleSource = source.value
        const cached = probed.get(source.value)
        if (cached) {
          eligible.value     = cached.eligible
          facetImpact.value  = cached.facetImpact
          lengthImpact.value = cached.lengthImpact
        } else {
          eligible.value     = null
          facetImpact.value  = {}
          lengthImpact.value = null
          void probeSource(module, source.value)
        }
      }
    } catch (thrown) {
      if (thrown instanceof WebAssembly.RuntimeError) {
        // A panic poisons the instance, so drop it and bump the counter,
        // leaving the next format to instantiate a fresh module.
        module = null
        reinit += 1
        error.value = TRAP_NOTICE
      } else {
        error.value = String(errorMessage(thrown))
      }
    } finally {
      status.value = 'idle'
    }
  }

  // A returning reader restores their last source and config from the store,
  // so an accidental navigation away never discards an edited `prose.toml`.
  // A first visit with nothing saved seeds a random example instead.
  function store(): Storage | null {
    return typeof window === 'undefined' ? null : window.localStorage
  }

  function loadSaved(): { configToml: string, source: string } | null {
    const local = store()
    if (!local) return null
    try {
      const raw = local.getItem(STORAGE_KEY)
      return raw ? (JSON.parse(raw) as { configToml: string, source: string }) : null
    } catch {
      return null
    }
  }

  function save(): void {
    const local = store()
    if (!local) return
    try {
      local.setItem(
        STORAGE_KEY,
        JSON.stringify({ configToml: configToml.value, source: source.value })
      )
    } catch {
      // A full or blocked store just skips the save.
    }
  }

  // Builds a link reproducing the current session, compact when the source
  // is an untouched pool case, leaving the address bar itself alone.
  async function share(): Promise<string | null> {
    if (typeof window === 'undefined') return null
    const match = cases.find(entry => entry.source === source.value)
    const state: SharedState = match
      ? { case: match.id, configToml: configToml.value }
      : { configToml: configToml.value, source: source.value }
    const payload = await encodeShare(state)
    if (payload === null) return null
    return `${window.location.href.split('#')[0]}${HASH_PREFIX}${payload}`
  }

  function adopt(state: { configToml: string, source: string }): void {
    source.value     = state.source
    configToml.value = state.configToml
    try {
      parsed.value = state.configToml.trim() ? (parse(state.configToml) as ParsedConfig) : {}
    } catch {
      parsed.value = {}
    }
  }

  // A share link outranks the visitor's own saved session, which in turn
  // outranks seeding a fresh random example. The decode only defers the
  // format when a share payload is actually present.
  async function start(): Promise<void> {
    const hash   = typeof window === 'undefined' ? '' : window.location.hash
    const shared = hash.startsWith(HASH_PREFIX) ? await decodeShare(hash) : null
    let seed: { configToml: string, source: string } | null = null
    if (shared) {
      const src = shared.source ?? cases.find(entry => entry.id === shared.case)?.source
      if (src !== undefined) seed = { configToml: shared.configToml, source: src }
    }
    const saved = seed ?? loadSaved()
    if (saved) {
      adopt(saved)
    } else {
      activeIndex.value = pick(cases.length, activeIndex.value)
      source.value      = activeCase.value.source
    }
    await format()
  }

  // A fresh example clears every edit: a different case seeds the source
  // and the config resets to its defaults.
  function refresh(): void {
    activeIndex.value = pick(cases.length, activeIndex.value)
    source.value      = activeCase.value.source
    parsed.value      = {}
    configToml.value  = ''
  }

  watchDebounced(configToml, text => {
    try {
      parsed.value      = text.trim() ? (parse(text) as ParsedConfig) : {}
      configError.value = ''
    } catch (thrown) {
      configError.value = String(errorMessage(thrown))
    }
  }, { debounce: debounceMs })

  watchDebounced([source, configToml], () => { save(); format() }, { debounce: debounceMs })

  return {
    activeCase,
    caseCount: cases.length,
    codeLineLength,
    configError,
    configToml,
    diagnostics,
    eligible,
    error,
    facetImpact,
    facetValue,
    formatted,
    isMoved,
    lengthImpact,
    lengthValue,
    lengths: schema.lengths,
    refresh,
    rules: schema.rules,
    setFacet,
    setLength,
    share,
    source,
    start,
    status
  }
}
