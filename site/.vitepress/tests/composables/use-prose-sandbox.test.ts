// @vitest-environment happy-dom
import { flushPromises } from '@vue/test-utils'

import { useProseSandbox }                        from '../../lib/composables/use-prose-sandbox'
import type { ProseSandbox, ProseSandboxOptions } from '../../lib/composables/use-prose-sandbox'
import type { SandboxSchema }                     from '../../lib/sandbox/config-schema.data'
import type { ProseWasm }                         from '../../lib/sandbox/load-module'
import type { SandboxCase }                       from '../../lib/sandbox/pool.data'
import { encodeShare }                            from '../../lib/sandbox/share-link'
import { mountSetup }                             from '../dom'

type Formatter = ProseWasm['format']
type Loader    = (reinit: number) => Promise<ProseWasm>

const STORAGE_KEY = 'prose-sandbox'

// happy-dom omits `localStorage`, so each test runs against a fresh in-memory
// store standing in for it, exercising the composable's real `window` path.
function memoryStorage(): Storage {
  const map = new Map<string, string>()
  return {
    clear      : () => { map.clear() },
    getItem    : key => map.get(key) ?? null,
    key        : index => [...map.keys()][index] ?? null,
    get length() { return map.size },
    removeItem : key => { map.delete(key) },
    setItem    : (key, value) => { map.set(key, value) }
  } as Storage
}

const SCHEMA: SandboxSchema = {
  codeLineLength : 88,
  lengths: [
    { default: 88, key: 'code-line-length',      label: 'Code' },
    { default: 76, key: 'docstring-line-length', label: 'Docstring' }
  ],
  rules: [
    {
      family : 'alignment',
      slug   : 'align-equals',
      facets : [
        { default: true, hintHtml: '', key: 'enabled', kind: 'bool', label: 'Enabled' },
        { default: 16, hintHtml: 'The width-spread budget.', key: 'max-shift', kind: 'int', label: 'Max Shift' },
        { default: true, hintHtml: '', key: 'condense', kind: 'bool', label: 'Condense' },
        { default: [], hintHtml: '', key: 'allow-pattern', kind: 'stringList', label: 'Allow Pattern' }
      ]
    },
    {
      family : 'formatting',
      slug   : 'blank-lines',
      facets : [{ default: true, hintHtml: '', key: 'enabled', kind: 'bool', label: 'Enabled' }]
    }
  ]
}

const CASES: readonly SandboxCase[] = [
  { id: 'a', source: 'seed a', title: 'Case A' },
  { id: 'b', source: 'seed b', title: 'Case B' }
]

const ENABLED   = SCHEMA.rules[0].facets[0]
const MAX_SHIFT = SCHEMA.rules[0].facets[1]

const formatting = (formatted: string, diagnostics = '', firedRules: readonly string[] = []): Formatter =>
  () => ({ config: '', diagnostics, fired_rules: firedRules, formatted })

const moduleWith = (format: Formatter): ProseWasm => ({ default: () => Promise.resolve(), format })

const okLoader: Loader = () => Promise.resolve(moduleWith(formatting('OUT')))

const sandbox = (load: Loader, options: Partial<ProseSandboxOptions> = {}): ProseSandbox =>
  mountSetup(() => useProseSandbox({ cases: CASES, schema: SCHEMA, load, pick: () => 0, ...options }))

describe('useProseSandbox', () => {
  beforeEach(() => {
    Object.defineProperty(window, 'localStorage', { configurable: true, value: memoryStorage() })
    window.history.replaceState(null, '', window.location.pathname)
  })

  afterEach(() => { vi.useRealTimers() })

  it('starts on a picked case and reports the formatted output', async () => {
    const load = vi.fn<Loader>(okLoader)
    const api  = sandbox(load, { pick: () => 1 })
    await api.start()
    expect(api.source.value).toBe('seed b')
    expect(api.formatted.value).toBe('OUT')
    expect(api.error.value).toBe('')
    expect(load).toHaveBeenLastCalledWith(0)
  })

  it('loads the module once and reuses it across runs', async () => {
    const load = vi.fn<Loader>(okLoader)
    const api  = sandbox(load)
    await api.start()
    await api.start()
    expect(load).toHaveBeenCalledTimes(1)
  })

  it('shares one instantiation across concurrent cold formats', async () => {
    const load = vi.fn<Loader>(okLoader)
    const api  = sandbox(load)
    await Promise.all([api.start(), api.start()])
    expect(load).toHaveBeenCalledTimes(1)
  })

  it('parses the lint findings from the format result', async () => {
    const records = JSON.stringify([
      { code: 'bare-imports', end_location: { column: 2, row: 1 }, location: { column: 1, row: 1 }, message: 'm' }
    ])
    const api = sandbox(() => Promise.resolve(moduleWith(formatting('OUT', records))))
    await api.start()
    expect(api.diagnostics.value).toHaveLength(1)
    expect(api.diagnostics.value[0].code).toBe('bare-imports')
  })

  it('computes the eligible rule set from the default run on the source', async () => {
    const fired = ['align-equals', 'blank-lines']
    const api = sandbox(() => Promise.resolve(moduleWith(formatting('OUT', '', fired))))
    await api.start()
    expect(api.eligible.value).toEqual(['align-equals', 'blank-lines'])
  })

  it('probes each eligible rule for the facets that can affect the source', async () => {
    // Only the `max-shift = 1` extreme changes the output, so the int facet
    // has impact, the flipped bool does not, and the unprobeable string list
    // fails open.
    const format: Formatter = config => ({
      config      : '',
      diagnostics : '',
      fired_rules : ['align-equals', 'blank-lines'],
      formatted   : config.includes('max-shift = 1') ? 'SHIFTED' : 'OUT'
    })
    const api = sandbox(() => Promise.resolve(moduleWith(format)))
    await api.start()
    await vi.waitFor(() => {
      expect(api.facetImpact.value['align-equals']).toEqual(['max-shift', 'allow-pattern'])
      expect(api.facetImpact.value['blank-lines']).toEqual([])
    })
  })

  it('counts a bool facet as impactful when its flip changes the findings', async () => {
    const format: Formatter = config => ({
      config      : '',
      diagnostics : config.includes('condense = false') ? '[{"code":"x"}]' : '',
      fired_rules : ['align-equals'],
      formatted   : 'OUT'
    })
    const api = sandbox(() => Promise.resolve(moduleWith(format)))
    await api.start()
    await vi.waitFor(() => {
      expect(api.facetImpact.value['align-equals']).toContain('condense')
    })
    expect(api.facetImpact.value).not.toHaveProperty('blank-lines')
  })

  it('fails a facet probe open when its run throws', async () => {
    const format: Formatter = config => {
      if (config.includes('max-shift')) throw new Error('bad config')
      return { config: '', diagnostics: '', fired_rules: ['align-equals'], formatted: 'OUT' }
    }
    const api = sandbox(() => Promise.resolve(moduleWith(format)))
    await api.start()
    await vi.waitFor(() => {
      expect(api.facetImpact.value['align-equals']).toContain('max-shift')
    })
  })

  it('keeps a good format when the default-config probe run traps', async () => {
    window.localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ configToml: 'code-line-length = 40\n', source: 'saved source' })
    )
    const format: Formatter = config => {
      if (config === '') throw new WebAssembly.RuntimeError('unreachable')
      return { config: '', diagnostics: '', fired_rules: [], formatted: 'OUT' }
    }
    const api = sandbox(() => Promise.resolve(moduleWith(format)))
    await api.start()
    expect(api.formatted.value).toBe('OUT')
    expect(api.error.value).toBe('')
    await vi.waitFor(() => expect(api.eligible.value).toEqual([]))
  })

  it('probes the length knobs and keeps only the impactful ones', async () => {
    const format: Formatter = config => ({
      config      : '',
      diagnostics : '',
      fired_rules : [],
      formatted   : config.includes('code-line-length = 30') ? 'NARROW' : 'OUT'
    })
    const api = sandbox(() => Promise.resolve(moduleWith(format)))
    await api.start()
    await vi.waitFor(() => expect(api.lengthImpact.value).toEqual(['code-line-length']))
  })

  it('caches the probe results per source and replays them without new runs', async () => {
    const format = vi.fn<Formatter>((config, src) => ({
      config      : '',
      diagnostics : '',
      fired_rules : src === 'seed a' ? ['align-equals'] : ['blank-lines'],
      formatted   : 'OUT'
    }))
    const api = sandbox(() => Promise.resolve(moduleWith(format)), { debounceMs: 5 })
    const probeRuns = () =>
      format.mock.calls.filter(call => call[0].includes('align-equals')).length
    await api.start()
    await vi.waitFor(() => expect(api.facetImpact.value['align-equals']).toBeDefined())
    const initialProbes = probeRuns()
    expect(initialProbes).toBeGreaterThan(0)
    api.source.value = 'seed b'
    await vi.waitFor(() => expect(api.facetImpact.value['blank-lines']).toEqual([]))
    api.source.value = 'seed a'
    await vi.waitFor(() => expect(api.facetImpact.value['align-equals']).toBeDefined())
    expect(probeRuns()).toBe(initialProbes)
  })

  it('surfaces a parse or config error and keeps the instance', async () => {
    const load = vi.fn<Loader>(() =>
      Promise.resolve(moduleWith(() => { throw new Error('no parse') })))
    const api = sandbox(load)
    await api.start()
    expect(api.error.value).toBe('no parse')
    await api.start()
    expect(load).toHaveBeenCalledTimes(1)
  })

  it('renders a non-Error throw as its string form', async () => {
    // oxlint-disable-next-line no-throw-literal -- exercises the non-Error catch branch
    const load: Loader = () => Promise.resolve(moduleWith(() => { throw 'raw failure' }))
    const api = sandbox(load)
    await api.start()
    expect(api.error.value).toBe('raw failure')
  })

  it('surfaces a module-load failure without crashing', async () => {
    const load: Loader = () => Promise.reject(new Error('module offline'))
    const api = sandbox(load)
    await api.start()
    expect(api.error.value).toBe('module offline')
  })

  it('recovers from a panic trap by re-instantiating a fresh module', async () => {
    const trap    = moduleWith(() => { throw new WebAssembly.RuntimeError('unreachable') })
    const healthy = moduleWith(formatting('RECOVERED'))
    const load    = vi.fn<Loader>(reinit => Promise.resolve(reinit === 0 ? trap : healthy))
    const api     = sandbox(load)
    await api.start()
    expect(api.error.value).toMatch(/internal error/)
    await api.start()
    expect(api.formatted.value).toBe('RECOVERED')
    expect(api.error.value).toBe('')
    expect(load).toHaveBeenNthCalledWith(1, 0)
    expect(load).toHaveBeenNthCalledWith(2, 1)
  })

  it('refresh moves to a different case', () => {
    const api = sandbox(okLoader, { pick: () => 1 })
    api.refresh()
    expect(api.source.value).toBe('seed b')
  })

  it('reads a facet default before any override', () => {
    const api = sandbox(okLoader)
    expect(api.facetValue('align-equals', ENABLED)).toBe(true)
    expect(api.facetValue('align-equals', MAX_SHIFT)).toBe(16)
  })

  it('reads a sub-facet value back from the written config', () => {
    const api = sandbox(okLoader)
    api.setFacet('align-equals', MAX_SHIFT, 4)
    expect(api.facetValue('align-equals', MAX_SHIFT)).toBe(4)
    expect(api.facetValue('align-equals', ENABLED)).toBe(true)
  })

  it('stays on the only case when the pool holds one', () => {
    const single = [{ id: 'x', source: 'lone', title: 'Only' }]
    const api = mountSetup(() =>
      useProseSandbox({ cases: single, schema: SCHEMA, load: okLoader }))
    api.refresh()
    expect(api.source.value).toBe('lone')
  })

  it('the default picker lands on a real, different case', () => {
    const api = mountSetup(() =>
      useProseSandbox({ cases: CASES, schema: SCHEMA, load: okLoader }))
    const before = api.source.value
    api.refresh()
    expect(CASES.map(c => c.source)).toContain(api.source.value)
    expect(api.source.value).not.toBe(before)
  })

  it('writes a disabled rule into the config toml', () => {
    const api = sandbox(okLoader)
    api.setFacet('align-equals', ENABLED, false)
    expect(api.configToml.value).toContain('align-equals = false')
    expect(api.facetValue('align-equals', ENABLED)).toBe(false)
  })

  it('writes a sub-facet override and clears it back to empty', () => {
    const api = sandbox(okLoader)
    api.setFacet('align-equals', MAX_SHIFT, 4)
    expect(api.configToml.value).toContain('max-shift = 4')
    api.setFacet('align-equals', MAX_SHIFT, 16)
    expect(api.configToml.value).toBe('')
  })

  it('re-enabling a disabled rule drops the override', () => {
    const api = sandbox(okLoader)
    api.setFacet('align-equals', ENABLED, false)
    api.setFacet('align-equals', ENABLED, true)
    expect(api.configToml.value).toBe('')
  })

  it('writes and clears the code line length override', () => {
    const api = sandbox(okLoader)
    api.setLength('code-line-length', 40)
    expect(api.configToml.value).toContain('code-line-length = 40')
    api.setLength('code-line-length', 88)
    expect(api.configToml.value).toBe('')
  })

  it('projects a pasted config back onto the controls', async () => {
    vi.useFakeTimers()
    const api = sandbox(okLoader, { debounceMs: 50 })
    api.configToml.value = 'rules.align-equals = false\n'
    await vi.advanceTimersByTimeAsync(50)
    expect(api.facetValue('align-equals', ENABLED)).toBe(false)
    expect(api.configError.value).toBe('')
  })

  it('reads a pasted table-form disable and re-enables around sibling overrides', async () => {
    vi.useFakeTimers()
    const api = sandbox(okLoader, { debounceMs: 50 })
    api.configToml.value = '[rules.align-equals]\nenabled = false\nmax-shift = 4\n'
    await vi.advanceTimersByTimeAsync(50)
    expect(api.facetValue('align-equals', ENABLED)).toBe(false)
    api.setFacet('align-equals', ENABLED, true)
    expect(api.configToml.value).toContain('max-shift = 4')
    expect(api.configToml.value).not.toContain('enabled')
    expect(api.facetValue('align-equals', ENABLED)).toBe(true)
  })

  it('reports a config error for unparseable toml', async () => {
    vi.useFakeTimers()
    const api = sandbox(okLoader, { debounceMs: 50 })
    api.configToml.value = 'this is = = not valid ['
    await vi.advanceTimersByTimeAsync(50)
    expect(api.configError.value).not.toBe('')
  })

  it('debounces rapid edits into a single format', async () => {
    vi.useFakeTimers()
    const format = vi.fn<Formatter>(formatting('OUT'))
    const load   = vi.fn<Loader>(() => Promise.resolve(moduleWith(format)))
    const api    = sandbox(load, { debounceMs: 50 })
    api.source.value = 'a'
    api.source.value = 'ab'
    api.source.value = 'abc'
    await vi.advanceTimersByTimeAsync(50)
    await flushPromises()
    // One settled cycle: the display run plus the eligibility run for the
    // new source, rather than a run per intermediate edit.
    expect(format).toHaveBeenCalledTimes(2)
    expect(api.formatted.value).toBe('OUT')
  })

  it('formats a rule toggle without waiting out the typing debounce', async () => {
    vi.useFakeTimers()
    const format = vi.fn<Formatter>(formatting('OUT'))
    const api    = sandbox(() => Promise.resolve(moduleWith(format)), { debounceMs: 250 })
    api.setFacet('align-equals', ENABLED, false)
    api.setFacet('blank-lines', SCHEMA.rules[1].facets[0], false)
    await flushPromises()
    // The two toggles coalesce into one immediate display run, trailed only
    // by the first format's eligibility baseline, with no timer advance.
    expect(format).toHaveBeenCalledTimes(2)
    expect(format).toHaveBeenNthCalledWith(1, expect.stringContaining('align-equals = false'), 'seed a')
    expect(format).toHaveBeenNthCalledWith(1, expect.stringContaining('blank-lines = false'), 'seed a')
  })

  it('reads and writes a length knob and clears it back to default', () => {
    const api = sandbox(okLoader)
    expect(api.lengths).toEqual(SCHEMA.lengths)
    expect(api.lengthValue('docstring-line-length')).toBe(76)
    api.setLength('docstring-line-length', 70)
    expect(api.configToml.value).toContain('docstring-line-length = 70')
    expect(api.lengthValue('docstring-line-length')).toBe(70)
    api.setLength('docstring-line-length', 76)
    expect(api.configToml.value).toBe('')
  })

  it('falls back to the code line length for an unlisted knob', () => {
    const api = sandbox(okLoader)
    expect(api.lengthValue('import-line-length')).toBe(88)
  })

  it('builds a share link that restores the session ahead of the store', async () => {
    const first = sandbox(okLoader)
    await first.start()
    first.source.value = 'shared source'
    const url = await first.share()
    expect(url).toMatch(/#1\./)
    expect(window.location.hash).toBe('')
    window.localStorage.clear()
    window.history.replaceState(null, '', url?.slice(url.indexOf('#')))
    const second = sandbox(okLoader)
    await second.start()
    expect(second.source.value).toBe('shared source')
  })

  it('shares an untouched pool case by its id and restores it with the config', async () => {
    const first = sandbox(okLoader)
    await first.start()
    first.setFacet('align-equals', ENABLED, false)
    const url = await first.share()
    window.localStorage.clear()
    window.history.replaceState(null, '', url?.slice(url.indexOf('#')))
    const second = sandbox(okLoader, { pick: () => 1 })
    await second.start()
    expect(second.source.value).toBe('seed a')
    expect(second.facetValue('align-equals', ENABLED)).toBe(false)
  })

  it('ignores an undecodable share hash and seeds normally', async () => {
    window.history.replaceState(null, '', '#1.not-a-real-payload')
    const api = sandbox(okLoader)
    await api.start()
    expect(api.source.value).toBe('seed a')
  })

  it('falls back to normal seeding when a share names an unknown case', async () => {
    const payload = await encodeShare({ case: 'ghost/renamed_away', configToml: '' })
    window.history.replaceState(null, '', `#1.${payload}`)
    const api = sandbox(okLoader)
    await api.start()
    expect(api.source.value).toBe('seed a')
  })

  it('restores the saved source and config on start', async () => {
    window.localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ configToml: 'code-line-length = 40\n', source: 'saved source' })
    )
    const api = sandbox(okLoader)
    await api.start()
    expect(api.source.value).toBe('saved source')
    expect(api.configToml.value).toBe('code-line-length = 40\n')
    expect(api.lengthValue('code-line-length')).toBe(40)
  })

  it('tolerates a corrupt saved config without throwing', async () => {
    window.localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ configToml: 'not = = valid [', source: 'saved source' })
    )
    const api = sandbox(okLoader)
    await api.start()
    expect(api.source.value).toBe('saved source')
  })

  it('ignores an unreadable store and seeds a fresh example', async () => {
    window.localStorage.setItem(STORAGE_KEY, '{ not json')
    const api = sandbox(okLoader)
    await api.start()
    expect(api.source.value).toBe('seed a')
  })

  it('persists an edit to the store', async () => {
    vi.useFakeTimers()
    const api = sandbox(okLoader, { debounceMs: 50 })
    api.setFacet('align-equals', ENABLED, false)
    api.source.value = 'edited'
    await vi.advanceTimersByTimeAsync(50)
    const saved = JSON.parse(window.localStorage.getItem(STORAGE_KEY) ?? '{}')
    expect(saved.source).toBe('edited')
    expect(saved.configToml).toContain('align-equals = false')
  })

  it('runs without a store present', async () => {
    vi.useFakeTimers()
    const descriptor = Object.getOwnPropertyDescriptor(window, 'localStorage')
    Object.defineProperty(window, 'localStorage', { configurable: true, value: undefined })
    const api = sandbox(okLoader, { debounceMs: 50 })
    await api.start()
    api.source.value = 'x'
    await vi.advanceTimersByTimeAsync(50)
    expect(api.source.value).toBe('x')
    if (descriptor) Object.defineProperty(window, 'localStorage', descriptor)
    else delete (window as { localStorage?: unknown }).localStorage
  })
})
