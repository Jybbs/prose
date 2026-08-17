// @vitest-environment happy-dom
import { useHashOpen } from '../../lib/composables/use-hash-open'
import { mountSetup }  from '../dom'

describe('useHashOpen', () => {
  afterEach(() => { window.location.hash = '' })

  it('reports the bare fragment the address bar already carries', () => {
    window.location.hash = '#dict_expand_sort_align'
    const seen = vi.fn<(fragment: string) => void>()
    mountSetup(() => useHashOpen(seen))
    expect(seen).toHaveBeenCalledWith('dict_expand_sort_align')
  })

  it('reports an empty fragment where the address bar carries no hash', () => {
    const seen = vi.fn<(fragment: string) => void>()
    mountSetup(() => useHashOpen(seen))
    expect(seen).toHaveBeenCalledWith('')
  })

  it('reports again on every later hash change', () => {
    const seen = vi.fn<(fragment: string) => void>()
    mountSetup(() => useHashOpen(seen))
    window.location.hash = '#arm_dicts_sort_expand_align'
    window.dispatchEvent(new Event('hashchange'))
    expect(seen).toHaveBeenLastCalledWith('arm_dicts_sort_expand_align')
  })
})
