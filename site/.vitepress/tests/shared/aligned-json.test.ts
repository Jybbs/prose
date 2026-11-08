import { alignedJson } from '../../lib/shared/aligned-json'

describe('alignedJson', () => {
  it('aligns the colons of an object and keeps a nested scalar object on one row', () => {
    expect(alignedJson({ code: 'x', end_location: { column: 1, row: 2 } }))
      .toBe('{\n  "code"         : "x",\n  "end_location" : { "column": 1, "row": 2 }\n}')
  })

  it('breaks a nested object holding an object and an array of objects', () => {
    expect(alignedJson({ fix: { edits: [{ at: { row: 1 } }] } }))
      .toBe('{\n  "fix" : {\n    "edits" : [\n      {\n        "at" : { "row": 1 }\n      }\n    ]\n  }\n}')
  })

  it.each([[[], '[]'], [{}, '{}'], [null, 'null'], [{ a: [] }, '{\n  "a" : []\n}']])('renders %j as %j', (value, text) => {
    expect(alignedJson(value)).toBe(text)
  })
})
