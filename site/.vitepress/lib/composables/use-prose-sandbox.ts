import { StorageSerializers, useStorage, watchDebounced } from '@vueuse/core'
import { ref, type Ref }                                  from 'vue'

import type { LintFinding }           from '../fixtures/lint-findings'
import type * as configSchema         from '../sandbox/config-schema.data'
import { loadModule, type ProseWasm } from '../sandbox/load-module'
import type { SandboxCase }           from '../sandbox/pool.data'
import * as session                   from '../sandbox/session'
import { errorMessage }               from '../shared/error-message'
import { useSandboxConfig }           from './use-sandbox-config'
import { useSandboxProbe }            from './use-sandbox-probe'

type FacetValue = configSchema.FacetValue

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
  unstable     : Ref<readonly string[]>
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

// The sandbox's whole state model, composing the `prose.toml` config and the
// eligibility probe over the wasm formatter this file owns.
export function useProseSandbox(options: ProseSandboxOptions): ProseSandbox {
  const { cases, schema, debounceMs = 250, load = loadModule, pick = session.randomOther } = options

  const diagnostics = ref<readonly LintFinding[]>([])
  const error       = ref('')
  const formatted   = ref('')
  const source      = ref(cases[0].source)
  const unstable    = ref<readonly string[]>([])

  let activeIndex = 0
  let eagerQueued = false
  let reinit      = 0

  let loading: Promise<ProseWasm> | null = null
  let module: ProseWasm | null           = null

  const config = useSandboxConfig(schema, debounceMs, eagerFormat)
  const probe  = useSandboxProbe(schema, current => module === current)

  // A rule or switch toggle is a discrete action, so its format runs on the
  // next microtask instead of waiting out the typing debounce, with a
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

  // A cold start and the first debounced format overlap, so both would see a
  // null module and load the wasm twice. Concurrent callers share the one
  // instantiation in flight, and a failed load clears it so the next retries.
  function moduleReady(): Promise<ProseWasm> {
    loading ??= instantiate().catch(thrown => {
      loading = null
      throw thrown
    })
    return loading
  }

  // The display already succeeded by the time the probe syncs, so a fault in
  // the probe runs must not reset the module or surface an error over a good
  // format.
  async function format(): Promise<void> {
    try {
      module ??= await moduleReady()
      const result = module.format(config.configToml.value, source.value)
      formatted.value   = result.formatted
      diagnostics.value = result.diagnostics ? JSON.parse(result.diagnostics) : []
      unstable.value    = result.unstable_rules ?? []
      error.value       = ''
      probe.sync(module, source.value)
    } catch (thrown) {
      if (thrown instanceof WebAssembly.RuntimeError) {
        // A panic poisons the instance, so drop it and bump the counter,
        // leaving the next format to instantiate a fresh module.
        loading = null
        module  = null
        reinit += 1
        error.value = TRAP_NOTICE
      } else {
        error.value = String(errorMessage(thrown))
      }
    }
  }

  function seedCase(): void {
    activeIndex  = pick(cases.length, activeIndex)
    source.value = cases[activeIndex].source
  }

  // A fresh example clears every edit, so a different case seeds the source and
  // the config resets to its defaults.
  function refresh(): void {
    seedCase()
    config.reset()
  }

  function share(): Promise<string | null> {
    return session.shareUrl(cases, config.configToml.value, source.value)
  }

  // A returning reader restores their last source and config from the store, so
  // an accidental navigation away never discards an edited `prose.toml`. A
  // first visit with nothing saved seeds a random example instead.
  const saved = useStorage<session.SavedSession | null>(session.STORAGE_KEY, null, undefined, {
    listenToStorageChanges : false,
    serializer             : StorageSerializers.object,
    writeDefaults          : false
  })

  // A share link outranks the visitor's own saved session, which in turn
  // outranks seeding a fresh random example.
  async function start(): Promise<void> {
    const restored = (await session.sharedSeed(cases)) ?? saved.value
    if (restored) {
      source.value = restored.source
      config.adopt(restored.configToml)
    } else {
      seedCase()
    }
    await format()
  }

  watchDebounced([source, config.configToml], () => {
    saved.value = { configToml: config.configToml.value, source: source.value }
    format()
  }, { debounce: debounceMs })

  return {
    configError  : config.configError,
    configToml   : config.configToml,
    diagnostics  : diagnostics,
    eligible     : probe.eligible,
    error        : error,
    facetImpact  : probe.facetImpact,
    facetValue   : config.facetValue,
    formatted    : formatted,
    lengthImpact : probe.lengthImpact,
    lengthValue  : config.lengthValue,
    lengths      : schema.lengths,
    refresh      : refresh,
    rules        : schema.rules,
    setFacet     : config.setFacet,
    setLength    : config.setLength,
    share        : share,
    source       : source,
    start        : start,
    unstable     : unstable
  }
}
