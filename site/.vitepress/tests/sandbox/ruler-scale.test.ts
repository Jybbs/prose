import * as rulerScale from '../../lib/sandbox/ruler-scale'

describe('steppedLength', () => {
  it.each([
    { expected: 87,         key: 'ArrowDown',  shiftKey: false },
    { expected: 87,         key: 'ArrowLeft',  shiftKey: false },
    { expected: 89,         key: 'ArrowRight', shiftKey: false },
    { expected: 89,         key: 'ArrowUp',    shiftKey: false },
    { expected: 78,         key: 'ArrowLeft',  shiftKey: true  },
    { expected: 98,         key: 'ArrowRight', shiftKey: true  },
    { expected: 78,         key: 'PageDown',   shiftKey: false },
    { expected: 98,         key: 'PageUp',     shiftKey: false },
    { expected: rulerScale.LENGTH_MAX, key: 'End',        shiftKey: false },
    { expected: rulerScale.LENGTH_MIN, key: 'Home',       shiftKey: false }
  ])('steps 88 by $key with shift $shiftKey to $expected', ({ expected, key, shiftKey }) => {
    expect(rulerScale.steppedLength(key, shiftKey, 88)).toBe(expected)
  })

  it.each(['PageDown', 'PageUp'])('holds the ten-line %s step against shift', key => {
    expect(rulerScale.steppedLength(key, true, 88)).toBe(rulerScale.steppedLength(key, false, 88))
  })

  it('declines a key the rail does not handle', () => {
    expect(rulerScale.steppedLength('Tab', false, 88)).toBeNull()
  })
})
