export const PARTS = [
  { id: 'nib',     label: 'Pen-nib P' },
  { id: 'letters', label: 'Letters' },
  { id: 'tagline', label: 'Tagline' }
] as const

export type PartId = (typeof PARTS)[number]['id']

// Names each file the site serves a mark from beside the parts it draws, the
// last drawing every part in the lockup's own box.
export const MARK_FILES = {
  'logo.svg'               : ['nib'],
  'title.svg'              : ['nib', 'letters'],
  'title-with-tagline.svg' : ['nib', 'letters', 'tagline']
} as const satisfies Record<string, readonly PartId[]>

export type MarkFile = keyof typeof MARK_FILES

// Holds a drawing as the markup inside its `<svg>` and the box it is drawn in,
// each element in the markup wrapped in a group setting the fill it inherits.
export interface Drawing {
  markup  : string
  viewBox : string
}

export interface Marks {
  files : Record<MarkFile, Drawing>
  parts : Record<PartId, Drawing>
}

// Returns the file that draws exactly the `parts` turned on, if one does.
export function fileDrawing(parts: readonly PartId[]): MarkFile | undefined {
  return (Object.keys(MARK_FILES) as MarkFile[]).find(name =>
    MARK_FILES[name].length === parts.length && parts.every(part => (MARK_FILES[name] as readonly PartId[]).includes(part)))
}
