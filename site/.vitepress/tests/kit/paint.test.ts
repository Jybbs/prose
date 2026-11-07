// @vitest-environment happy-dom
import { millis, reducesMotion, snapInPlace } from '../../kit/drawing/paint'
import { domTest }                            from '../dom'

describe('millis', () => {
  it.each([['0.4s', 400], ['250ms', 250], ['0.4s, 0.15s', 400], ['120ms, 1s', 120]])(
    'reads %s as %i milliseconds',
    (time, ms) => { expect(millis(time)).toBe(ms) }
  )
})

describe('reducesMotion', () => {
  domTest('reads the device\'s motion setting', ({ reducedMotion }) => {
    expect(reducesMotion()).toBe(false)
    reducedMotion(true)
    expect(reducesMotion()).toBe(true)
  })
})

describe('snapInPlace', () => {
  it('drops the transition through the write and restores it after', () => {
    const el   = document.createElement('div')
    const seen = [] as boolean[]
    snapInPlace(el, true, () => { seen.push('snap' in el.dataset) })
    expect([seen, 'snap' in el.dataset]).toStrictEqual([[true], false])
  })

  it('writes with the transition in place where no snap is asked for', () => {
    const el   = document.createElement('div')
    const seen = [] as boolean[]
    snapInPlace(el, false, () => { seen.push('snap' in el.dataset) })
    expect(seen).toStrictEqual([false])
  })
})
