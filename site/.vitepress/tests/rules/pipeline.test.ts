import { parsePipelineJson, readPipeline } from '../../lib/rules/pipeline'

const ENTRY = { after: [], imperative: 'x', position: 1, preserves_tree: true, slug: 'a' }

function payloadWith(fields: object): string {
  return JSON.stringify([{ ...ENTRY, ...fields }])
}

describe('parsePipelineJson', () => {
  it('parses entries through the field validation', () => {
    expect(parsePipelineJson(payloadWith({ after: ['b'] }))).toStrictEqual([{ ...ENTRY, after: ['b'] }])
  })

  it.each([
    ['an empty array',                 '[]',                                       /emitted no pipeline entries/],
    ['a non-array payload',            '{}',                                       /emitted no pipeline entries/],
    ['a missing slug',                 payloadWith({ slug: undefined }),           /invalid or missing slug/],
    ['a missing position',             payloadWith({ position: undefined }),       /invalid or missing position/],
    ['a missing tree declaration',     payloadWith({ preserves_tree: undefined }), /invalid or missing preserves_tree/],
    ['a non-boolean tree declaration', payloadWith({ preserves_tree: 'true' }),    /invalid or missing preserves_tree/],
    ['a missing imperative',           payloadWith({ imperative: undefined }),     /invalid or missing imperative/],
    ['a missing after list',           payloadWith({ after: undefined }),          /invalid or missing after list/],
    ['a non-slug dependency',          payloadWith({ after: [7] }),                /dependency 0 is not a slug/]
  ])('rejects %s', (_name, payload, message) => {
    expect(() => parsePipelineJson(payload)).toThrow(message)
  })

  it('throws a TypeError when position is not a number', () => {
    expect(() => parsePipelineJson(payloadWith({ position: '1' }))).toThrow(TypeError)
  })
})

describe('readPipeline', () => {
  it('reads the built binary registry in pipeline order', () => {
    const pipeline = readPipeline(import.meta.url)
    expect(pipeline.length).toBeGreaterThan(0)
    expect(pipeline[0].position).toBe(1)
  })

  it('carries the declared dependencies, each naming an earlier rule', () => {
    const pipeline = readPipeline(import.meta.url)
    const position = new Map(pipeline.map((rule) => [rule.slug, rule.position]))
    for (const rule of pipeline) {
      for (const dependency of rule.after) {
        expect(position.get(dependency)).toBeLessThan(rule.position)
      }
    }
  })
})
