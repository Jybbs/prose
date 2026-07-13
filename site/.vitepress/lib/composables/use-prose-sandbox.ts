import { StorageSerializers, promiseTimeout, useStorage, watchDebounced } from '@vueuse/core'
import { parse, stringify }               from 'smol-toml'
import { computed, ref, toRaw, type Ref } from 'vue'

import type { LintFinding }           from '../fixtures/lint-findings'
import type * as configSchema         from '../sandbox/config-schema.data'
import { loadModule, type ProseWasm } from '../sandbox/load-module'
import type { SandboxCase }           from '../sandbox/pool.data'
import * as probe                     from '../sandbox/probe'
import * as shareLink                 from '../sandbox/share-link'
import { errorMessage }               from '../shared/error-message'

type FacetValue    = configSchema.FacetValue
type ParsedRule    = boolean | Record<string, FacetValue>
type ParsedConfig  = { rules?: Record<string, ParsedRule> } &
  Record<string, number | Record<string, ParsedRule> | undefined>
type SavedSession  = { configToml: string, source: string }
type SourceProbe   = {
  eligible     : readonly string[]
  facetImpact  : Record<string, readonly string[]>
  lengthImpact : readonly string[]
}

const STORAGE_KEY = 'prose-sandbox'
const TRAP_NOTICE =
  'The formatter hit an internal error on this input. Edit the source to try again.'

export interface ProseSandbox {
  configError  : Ref<string>
  configToml   : Ref<string>
  diagnostics  : Ref<readonly LintFinding[]>
  eligible     : Ref<readonly string[] | null>
  error        : Ref<string>
  facetImpact  : Ref<Record<string, readonly string[]>>
  facetValue   : (slug: string, facet: configSchema.Facet) => FacetValue
  formatted    : Ref<string>
  lengthImpact : Ref<readonly string[] | null>
  lengthValue  : (key: string) => number
  lengths      : readonly configSchema.LengthKnob[]
  refresh      : () => void
  rules        : readonly configSchema.RuleControl[]
  setFacet     : (slug: string, facet: configSchema.Facet, value: FacetValue) => void
  setLength    : (key: string, value: number) => void
  share        : () => Promise<string | null>
  source       : Ref<string>
  start        : () => Promise<void>
}

export interface ProseSandboxOptions {
  cases       : readonly SandboxCase[]
  schema      : configSchema.SandboxSchema
  debounceMs ?: number
  load       ?: (reinit: number) => Promise<ProseWasm>
  pick       ?: (count: number, exclude: number) => number
}

function randomOther(count: number, exclude: number): number {
  if (count <= 1) return 0
  const roll = Math.floor(Math.random() * (count - 1))
  return roll >= exclude ? roll + 1 : roll
}

// `structuredClone` rejects a reactive proxy, so unwrap to the raw
// object first.
function clone(config: ParsedConfig): ParsedConfig {
  return structuredClone(toRaw(config))
}

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
    let baseline: ReturnType<ProseWasm['format']>
    try {
      baseline = current.format('', target)
    } catch {
      eligible.value = []
      return
    }
    const fired = baseline.fired_rules
    eligible.value = fired
    await promiseTimeout(0)
    if (eligibleSource !== target) return
    const lengths = schema.lengths
      .filter(knob => probe.lengthHasImpact(baseline, current.format, knob.key, target))
      .map(knob => knob.key)
    lengthImpact.value = lengths
    const impact: Record<string, readonly string[]> = {}
    for (const rule of schema.rules) {
      if (!fired.includes(rule.slug)) continue
      await promiseTimeout(0)
      if (eligibleSource !== target) return
      impact[rule.slug] = rule.facets
        .filter(facet =>
          facet.key !== 'enabled' &&
          probe.facetHasImpact(baseline, facet, current.format, rule.slug, target))
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

  function facetValue(slug: string, facet: configSchema.Facet): FacetValue {
    const rule = parsed.value.rules?.[slug]
    if (facet.key === 'enabled') {
      if (rule === false) return false
      return typeof rule === 'object' ? rule.enabled ?? true : true
    }
    return typeof rule === 'object' ? rule[facet.key] ?? facet.default : facet.default
  }

  function setFacet(slug: string, facet: configSchema.Facet, value: FacetValue): void {
    const next = clone(parsed.value)
    const rules = (next.rules ??= {})
    const current = rules[slug]
    if (facet.key === 'enabled') {
      if (value === false) rules[slug] = false
      else if (current === false) delete rules[slug]
      else if (typeof current === 'object') delete current.enabled
      commit(next)
      eagerFormat()
      return
    }
    const table = typeof current === 'object' ? current : {}
    if (value === facet.default) delete table[facet.key]
    else table[facet.key] = value
    if (Object.keys(table).length === 0) delete rules[slug]
    else rules[slug] = table
    commit(next)
    if (facet.kind === 'bool') eagerFormat()
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

  let eagerQueued = false

  // A rule or switch toggle is a discrete action, so its format runs on
  // the next microtask instead of waiting out the typing debounce, with a
  // toggle-all burst coalescing into one run.
  function eagerFormat(): void {
    if (eagerQueued) return
    eagerQueued = true
    queueMicrotask(() => {
      eagerQueued = false
      void format()
    })
  }

  async function instantiate(): Promise<ProseWasm> {
    const next = await load(reinit)
    await next.default()
    return next
  }

  async function format(): Promise<void> {
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
    }
  }

  // A returning reader restores their last source and config from the store,
  // so an accidental navigation away never discards an edited `prose.toml`.
  // A first visit with nothing saved seeds a random example instead.
  const saved = useStorage<SavedSession | null>(STORAGE_KEY, null, undefined, {
    listenToStorageChanges : false,
    serializer             : StorageSerializers.object,
    writeDefaults          : false
  })

  // Builds a link reproducing the current session, compact when the source
  // is an untouched pool case, leaving the address bar itself alone.
  async function share(): Promise<string | null> {
    if (typeof window === 'undefined') return null
    const match = cases.find(entry => entry.source === source.value)
    const state: shareLink.SharedState = match
      ? { case: match.id, configToml: configToml.value }
      : { configToml: configToml.value, source: source.value }
    const payload = await shareLink.encodeShare(state)
    if (payload === null) return null
    return `${window.location.href.split('#')[0]}${shareLink.HASH_PREFIX}${payload}`
  }

  function adopt(state: SavedSession): void {
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
    const shared = hash.startsWith(shareLink.HASH_PREFIX) ? await shareLink.decodeShare(hash) : null
    let seed: SavedSession | null = null
    if (shared) {
      const src = shared.source ?? cases.find(entry => entry.id === shared.case)?.source
      if (src !== undefined) seed = { configToml: shared.configToml, source: src }
    }
    const session = seed ?? saved.value
    if (session) {
      adopt(session)
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

  watchDebounced([source, configToml], () => {
    saved.value = { configToml: configToml.value, source: source.value }
    format()
  }, { debounce: debounceMs })

  return {
    configError,
    configToml,
    diagnostics,
    eligible,
    error,
    facetImpact,
    facetValue,
    formatted,
    lengthImpact,
    lengthValue,
    lengths: schema.lengths,
    refresh,
    rules: schema.rules,
    setFacet,
    setLength,
    share,
    source,
    start
  }
}
