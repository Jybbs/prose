import { sfcFs } from '../../lib/shared/sfc-fs'

describe('sfcFs', () => {
  it('reads a file as UTF-8 text and reports whether one exists', () => {
    const file = import.meta.filename

    expect(sfcFs.fileExists(file)).toBe(true)
    expect(sfcFs.fileExists(`${file}.missing`)).toBe(false)
    expect(sfcFs.readFile(file)).toContain('sfcFs')
  })
})
