import { parsePipeline, parsePipelineSource } from '../lib/rules/pipeline-source'

const BLOCK = `register_rules! {
    "align-equals" : Foo : Bar => Baz => "Align consecutive assignments",
    "alphabetize"  : Foo : Bar => Baz => "Alphabetize sibling entries",
}`

describe('parsePipelineSource', () => {
  it('extracts ordered entries from the register block', () => {
    expect(parsePipelineSource(BLOCK)).toEqual([
      { imperative: 'Align consecutive assignments', position: 1, slug: 'align-equals' },
      { imperative: 'Alphabetize sibling entries',   position: 2, slug: 'alphabetize' }
    ])
  })

  it('throws when the register block is absent', () => {
    expect(() => parsePipelineSource('fn main() {}')).toThrow(/block not found/)
  })

  it('throws when the block parses zero rules', () => {
    expect(() => parsePipelineSource('register_rules! {\n}')).toThrow(/parsed zero rules/)
  })
})

describe('parsePipeline', () => {
  it('parses the crate rule source without throwing', () => {
    expect(parsePipeline(import.meta.url).length).toBeGreaterThan(0)
  })
})
