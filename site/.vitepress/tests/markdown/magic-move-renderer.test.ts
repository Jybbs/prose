// @vitest-environment happy-dom
import { MagicMoveRenderer } from '@shikijs/magic-move/renderer'

interface Token {
  content    : string
  key        : string
  offset     : number
  htmlStyle ?: Record<string, string>
}

const token = (key: string, content: string, htmlStyle?: Record<string, string>): Token =>
  ({ content, htmlStyle, key, offset: 0 })

const step = (...tokens: Token[]) => ({ tokens } as never)

// Counts the child rebuilds so a skipped one is observable, and stubs the
// animation lookup happy-dom does not implement so `render` reaches its end.
const mounted = () => {
  const container = document.createElement('pre')
  document.body.append(container)
  Object.assign(container, { getAnimations: () => [] })

  let rebuilds = 0
  const rebuild = container.replaceChildren.bind(container)
  container.replaceChildren = (...nodes: Node[]) => {
    rebuilds += 1
    rebuild(...nodes)
  }

  const renderer = new MagicMoveRenderer(container, { containerStyle: false, duration: 0 })
  return { container, rebuilds: () => rebuilds, renderer }
}

const RED  = { color: '#222' }
const BLUE = { color: '#00b' }

// One pass over the shapes the surface actually produces, an identical
// re-land, a restyle, a flip, the swap that follows it, a drop, and a
// substitution.
const SEQUENCE: ReadonlyArray<readonly ['render' | 'replace', ReturnType<typeof step>]> = [
  ['replace', step(token('a', 'x', RED), token('b', ' = 1'))],
  ['replace', step(token('a', 'x', RED), token('b', ' = 1'))],
  ['replace', step(token('a', 'x', BLUE), token('b', ' = 1'))],
  ['render',  step(token('a', 'x', BLUE), token('b', ' = 2'))],
  ['replace', step(token('a', 'x', BLUE), token('b', ' = 2'))],
  ['replace', step(token('a', 'x', BLUE))],
  ['replace', step(token('a', 'x', BLUE), token('c', ' = 3'))],
  ['replace', step(token('a', 'xx', BLUE), token('c', ' = 3'))]
]

const contents = (container: HTMLElement) =>
  [...container.children].slice(1).map(el => el.textContent)

describe('MagicMoveRenderer', () => {
  it('skips the child rebuild when the same step lands twice', () => {
    const { container, rebuilds, renderer } = mounted()
    const first = step(token('a', 'x'), token('b', ' = 1'))

    renderer.replace(first)
    expect(rebuilds()).toBe(1)
    const before = [...container.children]

    // The swap re-lands the children already mounted, so the identity walk
    // takes the whole call rather than re-inserting them.
    renderer.replace(step(token('a', 'x'), token('b', ' = 1')))
    expect(rebuilds()).toBe(1)
    expect([...container.children]).toEqual(before)
    expect(contents(container)).toEqual(['x', ' = 1'])
  })

  it('rebuilds when a token drops out, dropping the stranded element', () => {
    const { container, rebuilds, renderer } = mounted()

    renderer.replace(step(token('a', 'x'), token('b', ' = 1')))
    renderer.replace(step(token('a', 'x')))
    expect(rebuilds()).toBe(2)
    expect(contents(container)).toEqual(['x'])
  })

  it('rebuilds when one token is substituted for another of the same count', () => {
    const { container, rebuilds, renderer } = mounted()

    renderer.replace(step(token('a', 'x'), token('b', ' = 1')))
    renderer.replace(step(token('a', 'x'), token('c', ' = 2')))
    expect(rebuilds()).toBe(2)
    expect(contents(container)).toEqual(['x', ' = 2'])
  })

  it('reapplies style when a carried token restyles', () => {
    const { container, renderer } = mounted()

    renderer.replace(step(token('a', 'x', { color: '#111' })))
    renderer.replace(step(token('a', 'x', { color: '#222' })))
    expect((container.children[1] as HTMLElement).style.color).toBe('#222')
  })

  it('leaves a carried token alone when nothing about it changed', () => {
    const { container, renderer } = mounted()
    renderer.replace(step(token('a', 'x', { color: '#111' })))

    const el = container.children[1] as HTMLElement
    el.style.color = '#999'

    // A stash match skips the write, so the out-of-band colour survives.
    renderer.replace(step(token('a', 'x', { color: '#111' })))
    expect(el.style.color).toBe('#999')
  })

  it('draws what the unguarded renderer would draw at every step', async () => {
    const guarded = mounted()
    const plain   = mounted()

    // Both guard families answer through instance methods, so shadowing them
    // gives an unguarded renderer to compare against without a second build.
    Object.assign(plain.renderer, { matchesApplied: () => false, sameChildren: () => false })

    for (const [how, next] of SEQUENCE) {
      if (how === 'render') {
        await guarded.renderer.render(next)
        await plain.renderer.render(next)
      } else {
        guarded.renderer.replace(next)
        plain.renderer.replace(next)
      }
      expect(guarded.container.innerHTML).toBe(plain.container.innerHTML)
    }

    // A sequence that never diverges would pass vacuously, so the skips have
    // to have actually fired.
    expect(guarded.rebuilds()).toBeLessThan(plain.rebuilds())
  })

  it('leaves a token that neither moved nor restyled out of the choreography', () => {
    const { container, renderer } = mounted()
    renderer.replace(step(token('a', 'x', RED), token('b', ' = 1')))

    // Deliberately not awaited. The move class goes on during the render and
    // comes off when the transition resolves, so the check lands between them.
    void renderer.render(step(token('a', 'x', RED), token('b', ' = 2')))
    const [, still, changed] = [...container.children] as HTMLElement[]
    expect(still.className).not.toContain('shiki-magic-move-move')
    expect(changed.className).toContain('shiki-magic-move-move')
  })

  it('keeps the stash in step with what an animating render wrote', async () => {
    const { container, rebuilds, renderer } = mounted()
    renderer.replace(step(token('a', 'x'), token('b', ' = 1')))

    // `render` writes content and style through its own path, so a stash left
    // behind there would make the next swap rebuild everything it just drew.
    await renderer.render(step(token('a', 'x'), token('b', ' = 2')))
    const after = rebuilds()

    renderer.replace(step(token('a', 'x'), token('b', ' = 2')))
    expect(rebuilds()).toBe(after)
    expect(contents(container)).toEqual(['x', ' = 2'])
  })
})
