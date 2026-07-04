// @vitest-environment happy-dom
import { describe, expect, test } from 'vitest'

import { el } from '../../../src/lib/shared/dom/el'

describe('el', () => {
  test.each([
    { name: 'creates a tagged, classed node carrying text', className: 'card',  content: 'hi',      tag: 'div',  text: 'hi' },
    { name: 'leaves text empty when content is undefined',  className: 'chip',  content: undefined, tag: 'span', text: '' }
  ])('$name', ({ className, content, tag, text }) => {
    const node = el(className, tag as 'div', content)
    expect(node.tagName.toLowerCase()).toBe(tag)
    expect(node).toHaveClass(className)
    expect(node.textContent).toBe(text)
  })
})
