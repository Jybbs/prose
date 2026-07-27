import { parsePipelineJson, readPipeline } from '../../lib/rules/pipeline'

describe('parsePipelineJson', () => {
  it('parses entries through the field validation', () => {
    const payload = '[{"after":["align-colons"],"imperative":"align things","position":1,"slug":"align-equals"}]'
    expect(parsePipelineJson(payload)).toEqual([
      { after: ['align-colons'], imperative: 'align things', position: 1, slug: 'align-equals' }
    ])
  })

  it.each([
    ['an empty array',        '[]',                                                        /emitted no pipeline entries/],
    ['a non-array payload',   '{}',                                                        /emitted no pipeline entries/],
    ['a missing slug',        '[{"after":[],"imperative":"x","position":1}]',              /invalid or missing slug/],
    ['a missing position',    '[{"after":[],"imperative":"x","slug":"a"}]',                /invalid or missing position/],
    ['a missing imperative',  '[{"after":[],"position":1,"slug":"a"}]',                    /invalid or missing imperative/],
    ['a missing after list',  '[{"imperative":"x","position":1,"slug":"a"}]',              /invalid or missing after list/],
    ['a non-slug dependency', '[{"after":[7],"imperative":"x","position":1,"slug":"a"}]',  /dependency 0 is not a slug/]
  ])('rejects %s', (_name, payload, message) => {
    expect(() => parsePipelineJson(payload)).toThrow(message)
  })

  it('throws a TypeError when position is not a number', () => {
    expect(() => parsePipelineJson('[{"after":[],"imperative":"x","slug":"a"}]')).toThrow(TypeError)
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
