import { parsePipelineJson, readPipeline } from '../../lib/rules/pipeline'

describe('parsePipelineJson', () => {
  it('parses entries through the field validation', () => {
    const payload = '[{"imperative":"align things","position":1,"slug":"align-equals"}]'
    expect(parsePipelineJson(payload)).toEqual([
      { imperative: 'align things', position: 1, slug: 'align-equals' }
    ])
  })

  it.each([
    ['an empty array',       '[]',                                /emitted no pipeline entries/],
    ['a non-array payload',  '{}',                                /emitted no pipeline entries/],
    ['a missing slug',       '[{"imperative":"x","position":1}]', /invalid or missing slug/],
    ['a missing position',   '[{"imperative":"x","slug":"a"}]',   /invalid or missing position/],
    ['a missing imperative', '[{"position":1,"slug":"a"}]',       /invalid or missing imperative/]
  ])('rejects %s', (_name, payload, message) => {
    expect(() => parsePipelineJson(payload)).toThrow(message)
  })
})

describe('readPipeline', () => {
  it('reads the built binary registry in pipeline order', () => {
    const pipeline = readPipeline(import.meta.url)
    expect(pipeline.length).toBeGreaterThan(0)
    expect(pipeline[0].position).toBe(1)
  })
})
