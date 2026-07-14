// @vitest-environment happy-dom
import { offsetAt } from '../../lib/sandbox/caret'

// happy-dom resolves no caret from a point, so each case installs the caret
// api the browser would have exposed and asserts the offset math on it.
const caretApis = (position?: object, range?: object): void => {
  Object.defineProperty(document, 'caretPositionFromPoint', {
    configurable : true,
    value        : position && (() => position)
  })
  Object.defineProperty(document, 'caretRangeFromPoint', {
    configurable : true,
    value        : range && (() => range)
  })
}

const CASES = [
  {
    expected : 6,
    name     : 'a text node, counting the text before it',
    stub     : (root: HTMLElement) =>
      caretApis({ offset: 2, offsetNode: root.querySelectorAll('span')[1].firstChild! })
  },
  {
    expected : 4,
    name     : 'an element node, by child index rather than text',
    stub     : (root: HTMLElement) =>
      caretApis({ offset: 1, offsetNode: root.querySelector('code')! })
  },
  {
    expected : 0,
    name     : 'a node outside the root',
    stub     : () => caretApis({ offset: 0, offsetNode: document.body })
  },
  {
    expected : 6,
    name     : 'a legacy caret range when no caret position resolves',
    stub     : (root: HTMLElement) => caretApis(undefined, {
      startContainer : root.querySelectorAll('span')[1].firstChild!,
      startOffset    : 2
    })
  },
  {
    expected : 0,
    name     : 'no caret at all when the browser exposes neither api',
    stub     : () => caretApis()
  }
]

const codeRoot = (): HTMLElement => {
  const root = document.createElement('div')
  root.innerHTML = '<pre><code><span>def </span><span>run</span>():</code></pre>'
  return root
}

describe('offsetAt', () => {
  it.each(CASES)('resolves $name', ({ expected, stub }) => {
    const root = codeRoot()
    stub(root)
    expect(offsetAt(root, 0, 0)).toBe(expected)
  })
})
