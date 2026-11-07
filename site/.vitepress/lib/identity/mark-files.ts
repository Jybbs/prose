import { JSDOM } from 'jsdom'

import { MARK_FILES } from './marks'

import type { Drawing, MarkFile, Marks, PartId } from './marks'

const LOCKUP: MarkFile = 'title-with-tagline.svg'

function rootOf(svg: string): SVGSVGElement {
  return new JSDOM(svg, { contentType: 'image/svg+xml' }).window.document.documentElement as unknown as SVGSVGElement
}

// Returns an element's markup inside a group carrying the fill it inherits.
function filled(element: Element): string {
  const fill = element.closest('[fill]')?.getAttribute('fill')
  return `<g fill="${fill}">${element.outerHTML.replace(` xmlns="${element.namespaceURI}"`, '')}</g>`
}

// Splits the lockup into its parts, the pen-nib P being the first path its
// brand group holds, the letters the groups after it, and the tagline the
// group after the brand group.
export function splitLockup(svg: string): Record<PartId, Drawing> {
  const root              = rootOf(svg)
  const viewBox           = root.getAttribute('viewBox')!
  const [brand, tagline]  = [...root.children]
  const [nib, ...letters] = [...brand!.children]
  return {
    letters : { markup: letters.map(filled).join(''), viewBox },
    nib     : { markup: filled(nib!), viewBox },
    tagline : { markup: filled(tagline!), viewBox }
  }
}

// Reads a mark file as the drawing its root holds, each child wrapped in a group
// carrying the fill it inherits.
export function readDrawing(svg: string): Drawing {
  const root = rootOf(svg)
  return { markup: [...root.children].map(filled).join(''), viewBox: root.getAttribute('viewBox')! }
}

// Reads each mark file and the lockup's parts through `read`, which returns a
// file's text by its name.
export function readMarks(read: (file: MarkFile) => string): Marks {
  const names = Object.keys(MARK_FILES) as MarkFile[]
  return {
    files : Object.fromEntries(names.map(name => [name, readDrawing(read(name))])) as Record<MarkFile, Drawing>,
    parts : splitLockup(read(LOCKUP))
  }
}
