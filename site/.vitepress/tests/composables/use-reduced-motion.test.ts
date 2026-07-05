// @vitest-environment happy-dom
import { useReducedMotion } from '../../lib/composables/use-reduced-motion'
import { domTest }          from '../dom'

const mediaQueryList = (matches: boolean): MediaQueryList =>
  ({
    addEventListener    : vi.fn<() => void>(),
    matches,
    media               : '(prefers-reduced-motion: reduce)',
    removeEventListener : vi.fn<() => void>()
  }) as unknown as MediaQueryList

afterEach(() => {
  vi.restoreAllMocks()
})

describe('useReducedMotion', () => {
  it('asks for the reduced-motion preference and reports a match', () => {
    const spy = vi.spyOn(window, 'matchMedia').mockReturnValue(mediaQueryList(true))
    expect(useReducedMotion().value).toBe(true)
    expect(spy).toHaveBeenCalledWith('(prefers-reduced-motion: reduce)')
  })

  it('reports false when the preference is unset', () => {
    vi.spyOn(window, 'matchMedia').mockReturnValue(mediaQueryList(false))
    expect(useReducedMotion().value).toBe(false)
  })

  domTest('follows the happy-dom device setting', ({ reducedMotion }) => {
    reducedMotion(true)
    expect(useReducedMotion().value).toBe(true)
  })
})
