import fs from 'node:fs'

import { readDrawing, readMarks, splitLockup } from '../../lib/identity/mark-files'
import { fileDrawing, MARK_FILES, PARTS }      from '../../lib/identity/marks'

import type { MarkFile, PartId } from '../../lib/identity/marks'

const read  = (file: MarkFile): string => fs.readFileSync(new URL(`../../../public/${file}`, import.meta.url), 'utf8')
const marks = readMarks(read)

describe('the mark files', () => {
  it('splits the lockup into a P, its letters, and its tagline, each in its own fill', () => {
    const parts = splitLockup(read('title-with-tagline.svg'))
    expect(Object.keys(parts).toSorted()).toStrictEqual(PARTS.map(part => part.id).toSorted())
    expect(parts.nib.markup).toMatch(/^<g fill="#[\da-f]{6}"><path [^>]*fill-rule="evenodd"/)
    expect(parts.tagline.markup).not.toContain(parts.nib.markup.slice(0, 18))
  })

  it.each(Object.entries(MARK_FILES).filter(([name]) => name !== 'logo.svg'))(
    '%s draws exactly the lockup parts it names, in the lockup box',
    (name, parts) => {
      const drawing = marks.files[name as MarkFile]
      expect(drawing.viewBox).toBe(marks.parts.nib.viewBox)
      const paths = (markup: string): string[] => markup.match(/<path [^>]*>/g)!
      expect(paths(drawing.markup)).toStrictEqual(paths((parts as readonly PartId[]).map(part => marks.parts[part].markup).join('')))
    }
  )

  it('reads logo.svg as the P alone in its own box', () => {
    const logo = readDrawing(read('logo.svg'))
    expect(logo.markup.match(/<path /g)).toHaveLength(1)
    expect(logo.viewBox).not.toBe(marks.parts.nib.viewBox)
  })
})

describe('fileDrawing', () => {
  it.each(Object.entries(MARK_FILES))('names %s for its parts in any order', (name, parts) => {
    expect(fileDrawing([...parts].toReversed())).toBe(name)
  })

  it.each([[['letters']], [['nib', 'tagline']], [[]]] as PartId[][][])('names no file for %j', parts => {
    expect(fileDrawing(parts)).toBeUndefined()
  })
})
