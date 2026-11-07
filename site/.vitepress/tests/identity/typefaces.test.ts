// @vitest-environment happy-dom
import { readType } from '../../lib/identity/typefaces'

describe('readType', () => {
  afterEach(() => { document.head.innerHTML = ''; document.body.innerHTML = '' })

  it('reads the computed type and the tokens of the declarations that set it, skipping a media rule whose query fails and a rule holding no style rule', () => {
    document.head.innerHTML = `<style>
      @font-face { font-family: "Lora Variable"; src: local("Lora"); }
      :root { --face: "Lora Variable", serif; --heavy: 700; --light: 300; --size: 20px; --slant: italic; --track: 2px; --wide: 2px; }
      .page .title { font-weight: var(--heavy); }
      .title { font-family: var(--face); font-size: var(--size); font-weight: var(--light); }
      @media (min-width: 1px) { .title { letter-spacing: var(--track); } }
      @media (min-width: 99999px) { .title { letter-spacing: var(--wide); } }
      @supports (display: grid) { .title { font-style: var(--slant); } }
    </style>`
    document.body.innerHTML = '<div class="page"><h1 class="title">Title</h1></div>'
    const reading = readType(document.querySelector('h1')!)
    expect(reading).toMatchObject({ face: 'Lora Variable', family: '"Lora Variable", serif', selector: '.title', size: 20, weight: '700' })
    expect(reading.tokens).toStrictEqual(['--face', '--size', '--slant', '--heavy', '--track'])
  })

  it('leaves the probe it measures with out of the page, naming the element where no rule sets its family', () => {
    document.body.innerHTML = '<p>Body</p>'
    expect(readType(document.querySelector('p')!).selector).toBe('p')
    expect(document.body.querySelectorAll('p')).toHaveLength(1)
  })
})
