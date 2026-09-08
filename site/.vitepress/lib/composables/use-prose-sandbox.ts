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
  configError     : Ref<string>
  configNotices   : Ref<readonly string[]>
  configToml      : Ref<string>
  diagnostics     : Ref<readonly LintFinding[]>
  drawn           : Ref<number>
  eligible        : Ref<readonly string[] | null>
  error           : Ref<string>
  facetImpact     : Ref<Record<string, readonly string[]>>
  facetValue      : (slug: string, facet: configSchema.Facet) => FacetValue
  formatNow       : () => void
  formatted       : Ref<string>
  lengthImpact    : Ref<readonly string[] | null>
  lengths         : readonly configSchema.LengthKnob[]
  lengthValue     : (key: string) => number
  refresh         : () => void
  renameConfigKey : (path: string, to: string) => void
  rules           : readonly configSchema.RuleControl[]
  setFacet        : (slug: string, facet: configSchema.Facet, value: FacetValue) => void
  setLength       : (key: string, value: number) => void
  share           : () => Promise<string | null>
  source          : Ref<string>
  start           : () => Promise<void>
  unstable        : Ref<readonly string[]>

}

export interface ProseSandboxOptions {
  cases       : readonly SandboxCase[]
  schema      : configSchema.SandboxSchema
  debounceMs ?: number
  load       ?: () => Promise<ProseWasm>
  pick       ?: (count: number, exclude: number) => number
}

// The sandbox's whole state model, composing the `prose.toml` config and the
// eligibility probe over the wasm formatter this file owns.
export function useProseSandbox(options: ProseSandboxOptions): ProseSandbox {
  const { cases, schema, debounceMs = 250, load = loadModule, pick = session.randomOther } = options

  const configNotices = ref<readonly string[]>([])
  const diagnostics   = ref<readonly LintFinding[]>([])
  const drawn         = ref(0)
  const error         = ref('')
  const formatted     = ref('')
  const source        = ref(cases[0].source)
  const unstable      = ref<readonly string[]>([])

  let activeIndex = 0
  let eagerQueued = false

  let ready: Promise<ProseWasm> | null       = null
  let published: session.SavedSession | null = null

  const config = useSandboxConfig(schema, debounceMs, formatNow)
  const probe  = useSandboxProbe(schema)

  // A rule toggle, a draw, and an applied edit are discrete actions, so their
  // formats run on the next microtask instead of waiting out the typing
  // debounce, with a toggle-all burst coalescing into one run.
  function formatNow(): void {
    if (eagerQueued) return
    eagerQueued = true
    queueMicrotask(() => {
      eagerQueued = false
      void format()
    })
  }

  async function instantiate(): Promise<ProseWasm> {
    const wasm = await load()
    await wasm.default()
    return wasm
  }

  // A cold start and the first debounced format overlap, so both would load the
  // wasm twice. Concurrent callers share the one instantiation in flight, and a
  // failed load clears it so the next retries.
  function moduleReady(): Promise<ProseWasm> {
    ready ??= instantiate().catch(thrown => {
      ready = null
      throw thrown
    })
    return ready
  }

  // A toggle formats eagerly and then again off the debounced watcher, so a
  // run over the pair already published returns early rather than re-parsing
  // `diagnostics` into a fresh array identity that would fire the surface's
  // watch mid-morph. The display already succeeded by the time the probe
  // syncs, so a fault in the probe runs must not reset the module or surface
  // an error over a good format.
  async function format(): Promise<void> {
    let wasm: ProseWasm | null = null
    try {
      wasm = await moduleReady()
      const configToml = config.configToml.value
      const text       = source.value
      if (published?.configToml === configToml && published.source === text) return
      const result = wasm.format(configToml, text, true)
      formatted.value     = result.formatted
      configNotices.value = result.config_notices
      diagnostics.value   = result.diagnostics
      unstable.value      = result.unstable_rules
      error.value         = ''
      published           = { configToml, source: text }
      probe.sync(wasm, text)
    } catch (thrown) {
      published = null
      // The notices come from the config that just failed, so they clear while
      // the formatted text and its findings hold their last good run.
      configNotices.value = []
      if (thrown instanceof WebAssembly.RuntimeError) {
        // A panic poisons the instance, so the glue rebuilds it in place and
        // the next format runs against fresh memory.
        wasm?.__wbg_reset_state()
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
    drawn.value += 1
    formatNow()
  }

  function share(): Promise<string | null> {
    return session.shareUrl(cases, config.configToml.value, source.value)
  }

  // A returning reader restores their last source and config from the store, so
  // an accidental navigation away never discards an edited `prose.toml`. A
  // first visit with nothing saved seeds a random example instead.
  const saved = useStorage<session.SavedSession | null>(session.STORAGE_KEY, null, undefined, {
    listenToStorageChanges : false,
    onError                : noteUnreadableSession,
    serializer             : StorageSerializers.object,
    writeDefaults          : false
  })

  // Drops a stored session that no longer deserializes, logging its message
  // alone.
  function noteUnreadableSession(cause: unknown): void {
    console.warn(`[sandbox] ignoring an unreadable saved session, ${errorMessage(cause)}`)
  }

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
    configError     : config.configError,
    configNotices   : configNotices,
    configToml      : config.configToml,
    diagnostics     : diagnostics,
    drawn           : drawn,
    eligible        : probe.eligible,
    error           : error,
    facetImpact     : probe.facetImpact,
    facetValue      : config.facetValue,
    formatNow       : formatNow,
    formatted       : formatted,
    lengthImpact    : probe.lengthImpact,
    lengthValue     : config.lengthValue,
    lengths         : schema.lengths,
    refresh         : refresh,
    renameConfigKey : config.renameKey,
    rules           : schema.rules,
    setFacet        : config.setFacet,
    setLength       : config.setLength,
    share           : share,
    source          : source,
    start           : start,
    unstable        : unstable
  }
}
