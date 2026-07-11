// @vitest-environment happy-dom
import { flushPromises } from '@vue/test-utils'

import { useProseSandbox }                        from '../../lib/composables/use-prose-sandbox'
import type { ProseSandbox, ProseSandboxOptions } from '../../lib/composables/use-prose-sandbox'
import type { ProseWasm }                         from '../../lib/sandbox/load-module'
import { mountSetup }                             from '../dom'

type Formatter = ProseWasm['format']
type Loader    = (reinit: number) => Promise<ProseWasm>

const formatting = (formatted: string): Formatter => () => ({ config: '', formatted })

const moduleWith = (format: Formatter): ProseWasm => ({ default: () => Promise.resolve(), format })

const sandbox = (load: Loader, options: Partial<ProseSandboxOptions> = {}): ProseSandbox =>
  mountSetup(() => useProseSandbox({ load, source: 'seed', ...options }))

describe('useProseSandbox', () => {
  it('formats the source and reports the output', async () => {
    const load = vi.fn<Loader>(() => Promise.resolve(moduleWith(formatting('OUT'))))
    const api  = sandbox(load)
    await api.format()
    expect(api.output.value).toBe('OUT')
    expect(api.error.value).toBe('')
    expect(api.status.value).toBe('idle')
    expect(load).toHaveBeenCalledTimes(1)
    expect(load).toHaveBeenLastCalledWith(0)
  })

  it('loads the module once and reuses it across runs', async () => {
    const load = vi.fn<Loader>(() => Promise.resolve(moduleWith(formatting('OUT'))))
    const api  = sandbox(load)
    await api.format()
    await api.format()
    expect(load).toHaveBeenCalledTimes(1)
  })

  it('surfaces a parse or config error and keeps the instance', async () => {
    const load = vi.fn<Loader>(() => Promise.resolve(moduleWith(() => { throw new Error('no parse') })))
    const api  = sandbox(load)
    await api.format()
    expect(api.error.value).toBe('no parse')
    await api.format()
    expect(load).toHaveBeenCalledTimes(1)
  })

  it('renders a non-Error throw as its string form', async () => {
    // oxlint-disable-next-line no-throw-literal -- exercises the non-Error catch branch
    const load: Loader = () => Promise.resolve(moduleWith(() => { throw 'raw failure' }))
    const api = sandbox(load)
    await api.format()
    expect(api.error.value).toBe('raw failure')
  })

  it('surfaces a module-load failure without crashing', async () => {
    const load: Loader = () => Promise.reject(new Error('module offline'))
    const api = sandbox(load)
    await api.format()
    expect(api.error.value).toBe('module offline')
    expect(api.status.value).toBe('idle')
  })

  it('recovers from a panic trap by re-instantiating a fresh module', async () => {
    const trap    = moduleWith(() => { throw new WebAssembly.RuntimeError('unreachable') })
    const healthy = moduleWith(formatting('RECOVERED'))
    const load    = vi.fn<Loader>(reinit => Promise.resolve(reinit === 0 ? trap : healthy))
    const api     = sandbox(load)
    await api.format()
    expect(api.error.value).toMatch(/internal error/)
    await api.format()
    expect(api.output.value).toBe('RECOVERED')
    expect(api.error.value).toBe('')
    expect(load).toHaveBeenNthCalledWith(1, 0)
    expect(load).toHaveBeenNthCalledWith(2, 1)
  })

  it('reports the loading status until the module resolves', async () => {
    let release!: (module: ProseWasm) => void
    const load: Loader = () => new Promise(resolve => { release = resolve })
    const api = sandbox(load)
    const pending = api.format()
    expect(api.status.value).toBe('loading')
    release(moduleWith(formatting('OUT')))
    await pending
    expect(api.status.value).toBe('idle')
    expect(api.output.value).toBe('OUT')
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
    expect(format).toHaveBeenCalledTimes(1)
    expect(api.output.value).toBe('OUT')
    vi.useRealTimers()
  })
})
