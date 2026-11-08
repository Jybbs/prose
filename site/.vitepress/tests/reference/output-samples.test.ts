import { FORMATS, splitRecords } from '../../lib/reference/output-samples'
import loader                    from '../../lib/reference/output-samples.data'

const samples = await loader.load([])

describe('output samples', () => {
  it('splits every format into one block per finding the json records list', () => {
    expect(samples.findings.length).toBeGreaterThan(1)
    for (const format of FORMATS) expect(samples.records[format]).toHaveLength(samples.findings.length)
  })

  it('draws each finding as a squiggle on its own copy of the input', () => {
    for (const finding of samples.findings) expect(finding.inputHtml).toContain(`data-rule="${finding.code}"`)
    expect(samples.inputHtml).not.toContain('data-rule')
  })

  it('renders the diff and the closing summary record', () => {
    expect(samples.diffHtml).toContain('+++')
    expect(samples.summary).toContain('schema_version')
  })
})

describe('splitRecords', () => {
  it('cuts the text format at each diagnostic header', () => {
    expect(splitRecords('text', 'warning: a\n  |\nerror: b\n')).toStrictEqual(['warning: a\n  |', 'error: b'])
  })

  it('aligns each json record and drops the summary', () => {
    const blocks = splitRecords('json', '{"kind":"diagnostic","code":"x"}\n{"kind":"summary"}\n')
    expect(blocks).toStrictEqual(['{\n  "kind" : "diagnostic",\n  "code" : "x"\n}'])
  })
})
