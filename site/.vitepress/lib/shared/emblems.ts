import type { WheelPlatform }                        from '../usage/platforms'
import { lookup }                                    from './lookup'
import { GLOSSARY_FAMILY_META, type GlossaryFamily } from './registries'

export interface Emblem {
  family ?: GlossaryFamily
  motif   : Motif
  name    : string
}

// Places a drawing on the 24-unit grid as `translate(x y) scale(scale)`, which
// sets its ink 18 units tall, no wider than 22, and centered on the grid.
export interface Fit {
  scale : number
  x     : number
  y     : number
}

// Each path is one subpath on a 24-unit grid. A shape is a silhouette, a cut
// is a detail inside one, and a line is a stroke outside every shape. `small`
// is the simpler drawing a badge takes below the nib's large size, where the
// full drawing loses its structure.
export interface Motif {
  cuts   : readonly string[]
  fit    : Fit
  label  : string
  lines  : readonly string[]
  shapes : readonly string[]
  small ?: SmallDrawing
}

// Draws a motif in one stroke, with `dots` as filled circles `[cx, cy, r]`
// and `heavy` as strokes at the nib's heavy weight.
interface SmallDrawing {
  dots  ?: readonly (readonly [number, number, number])[]
  fit    : Fit
  heavy ?: readonly string[]
  lines  : readonly string[]
}

const FAMILY_MOTIFS: Record<GlossaryFamily, Motif> = {
  alignment: {
    cuts   : [],
    fit    : { scale: 0.891, x: 1.3, y: 1.325 },
    label  : 'Ladder',
    lines  : ['M8 3v18', 'M16 3v18', 'M8 7.5h8', 'M8 12h8', 'M8 16.5h8'],
    shapes : []
  },
  cli: {
    cuts   : [],
    fit    : { scale: 0.932, x: 0.125, y: 1.5 },
    label  : 'Wand',
    lines  : ['M4 20 14 10', 'M7.5 3.5v3', 'M6 5h3'],
    shapes : ['M17 2.5 18.2 5.8 21.5 7 18.2 8.2 17 11.5 15.8 8.2 12.5 7 15.8 5.8Z'],
    small  : {
      fit   : { scale: 0.87, x: 1.3, y: 1.825 },
      heavy : ['M3.5 20.5 10.2 13.8'],
      lines : ['M15.5 2.5 17.6 6.4 21.5 8.5 17.6 10.6 15.5 14.5 13.4 10.6 9.5 8.5 13.4 6.4Z']
    }
  },
  docs: {
    cuts   : ['M7 8v10a2 2 0 0 1-2 2', 'M10 8h8', 'M10 12h3v4h-3Z', 'M15.5 12H18', 'M15.5 16H18'],
    fit    : { scale: 1.019, x: -0.225, y: -0.225 },
    label  : 'Newspaper',
    lines  : [],
    shapes : ['M7 4h12a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V9a1 1 0 0 1 1-1h3Z'],
    small  : {
      fit   : { scale: 1.013, x: -0.15, y: -0.15 },
      lines : [
        'M7 4h12a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V9a1 1 0 0 1 1-1h3',
        'M7 4v14a2 2 0 0 1-2 2', 'M11 8h6.5', 'M11 12h6.5', 'M11 16h4'
      ]
    }
  },
  engine: {
    fit    : { scale: 1.019, x: -0.225, y: -0.225 },
    label  : 'Owl',
    lines  : [],
    shapes : ['M5 4l3.5 2.5h7L19 4v9a7 7 0 0 1-14 0Z'],

    cuts: [
      'M10.25 10.5a1.25 1.25 0 1 1-2.5 0 1.25 1.25 0 1 1 2.5 0Z',
      'M16.25 10.5a1.25 1.25 0 1 1-2.5 0 1.25 1.25 0 1 1 2.5 0Z', 'M11 13.5l1 1.5 1-1.5'
    ],

    small: {
      dots  : [[9.5, 11, 1.4], [14.5, 11, 1.4], [12, 14.2, 1.4]],
      fit   : { scale: 1.013, x: -0.15, y: -0.15 },
      lines : ['M5 4l3.5 2.5h7L19 4v9a7 7 0 0 1-14 0Z']
    }
  },
  formatting: {
    cuts   : ['M16.5 7.5 9 15', 'M13 11h3.5'],
    fit    : { scale: 1.019, x: -0.225, y: -0.225 },
    label  : 'Feather',
    lines  : ['M9 15 4 20'],
    shapes : ['M20 4c-6.5.5-11 5-12 11v1h1c6-1 10.5-5.5 11-12Z']
  },
  layout: {
    cuts   : ['M9 14v3.5', 'M12 14v3.5', 'M15 14v3.5'],
    fit    : { scale: 1.147, x: -1.775, y: -3.5 },
    label  : 'Basket',
    lines  : ['M3 11h18', 'M7.5 11a4.5 4.5 0 0 1 9 0'],
    shapes : ['M4.5 11h15l-1.5 8a2 2 0 0 1-2 1.5H8A2 2 0 0 1 6 19Z'],
    small  : {
      fit   : { scale: 1.143, x: -1.725, y: -3.45 },
      lines : [
        'M3 11h18', 'M7.5 11a4.5 4.5 0 0 1 9 0',
        'M4.5 11 6 19a2 2 0 0 0 2 1.5h8a2 2 0 0 0 2-1.5l1.5-8', 'M10 11v4.5', 'M14 11v4.5'
      ]
    }
  },
  lint: {
    fit    : { scale: 0.974, x: 0.075, y: 0.425 },
    label  : 'Yarn',
    lines  : ['M17.5 14.75c.7 2.6 1.8 4.6 3.5 5.5'],
    shapes : ['M18.5 11a7.5 7.5 0 1 1-15 0 7.5 7.5 0 1 1 15 0Z'],

    cuts: [
      'M6.5 5C9 8 11 12 11.5 18.4', 'M10.5 3.6c2.5 2.9 4.3 6.9 4.7 13.9',
      'M3.7 12.5c2.8-1 6.3 0 8.3 5.8'
    ]
  },
  ordering: {
    cuts   : ['M10 5v14.2', 'M13 5.6v8.5', 'M16 5.4V9'],
    fit    : { scale: 1.019, x: -0.725, y: -0.225 },
    label  : 'Harp',
    lines  : [],
    shapes : ['M6 20V5.5C9 3 13 8 19 4L9.5 20Z'],
    small  : {
      fit   : { scale: 1.013, x: -0.4, y: -0.15 },
      lines : ['M5 20V6C8.5 3 13 8 19.5 4L9 20Z', 'M9 5.1V20', 'M13 5.7v8.2']
    }
  }
}

const PLATFORM_MOTIFS: Record<string, Motif> = {
  Linux: {
    fit   : { scale: 0.864, x: 1.625, y: 1.85 },
    label : 'Penguin',
    lines : ['M7.5 21h3', 'M13.5 21h3'],

    cuts: [
      'M12 9.5c2.2 0 3.9 2.6 3.9 5.4 0 2.1-1.7 3.1-3.9 3.1s-3.9-1-3.9-3.1c0-2.8 1.7-5.4 3.9-5.4Z',
      'M11.3 5.8a.55.55 0 1 1-1.1 0 .55.55 0 1 1 1.1 0Z',
      'M13.8 5.8a.55.55 0 1 1-1.1 0 .55.55 0 1 1 1.1 0Z', 'M11.2 7.5l.8.8.8-.8'
    ],

    shapes: [
      'M12 2.5c2.5 0 4 1.9 4 4.4 0 1.2.6 2.2 1.5 3.4 1.3 1.6 2.2 3.4 2.2 5.4 0 2.6-2.9 3.8-7.7 3.8'
      + 's-7.7-1.2-7.7-3.8c0-2 .9-3.8 2.2-5.4.9-1.2 1.5-2.2 1.5-3.4 0-2.5 1.5-4.4 4-4.4Z'
    ],

    small: {
      dots  : [[12, 7.2, 1.4]],
      fit   : { scale: 0.952, x: 0.575, y: 1.525 },
      lines : [
        'M12 2.5c3 0 4.8 2 4.8 4.6 0 1.1.5 2 1.3 3.1 1.3 1.7 2 3.5 2 5.6 0 2.6-2.9 3.7-8.1 3.7'
        + 's-8.1-1.1-8.1-3.7c0-2.1.7-3.9 2-5.6.8-1.1 1.3-2 1.3-3.1 0-2.6 1.8-4.6 4.8-4.6Z',
        'M8.4 19.3C8 15 9.6 11.4 12 11.4s4 3.6 3.6 7.9'
      ]
    }
  },
  Windows: {
    cuts   : [],
    fit    : { scale: 1.019, x: -0.225, y: -0.225 },
    label  : 'Window',
    shapes : [],

    lines: [
      'M5 4h14a1.5 1.5 0 0 1 1.5 1.5v13A1.5 1.5 0 0 1 19 20H5'
      + 'a1.5 1.5 0 0 1-1.5-1.5v-13A1.5 1.5 0 0 1 5 4Z',
      'M12 4v16', 'M3.5 12h17'
    ]
  },
  macOS: {
    cuts  : [],
    fit   : { scale: 0.923, x: 0.925, y: 1.2 },
    label : 'Apple',
    lines : ['M12 7.5c0-2 .5-3.5 1.5-4.5'],

    shapes: [
      'M12 7.5c-1.5-1-3.5-1.5-5.2-.5C4.5 8.4 4 11.5 5 14.5s3 6 5 6c1 0 1.2-.5 2-.5'
      + 's1 .5 2 .5c2 0 4-3 5-6s.5-6.1-1.8-7.5C15.5 6 13.5 6.5 12 7.5Z',
      'M13.5 5.5c.5-2 2.5-3 4.5-2.5-.5 2-2.5 3-4.5 2.5Z'
    ],

    small: {
      fit   : { scale: 0.89, x: 1.325, y: 1.85 },
      lines : [
        'M12 7.5c-1.5-1-3.5-1.5-5.2-.5C4.5 8.4 4 11.5 5 14.5s3 6 5 6c1 0 1.2-.5 2-.5'
        + 's1 .5 2 .5c2 0 4-3 5-6s.5-6.1-1.8-7.5C15.5 6 13.5 6.5 12 7.5Z',
        'M12 7.5c0-2 .5-3.5 1.5-4.5', 'M12.4 4.8c1-1.8 2.9-2.7 5.1-2.5'
      ]
    }
  }
}

export const MOTIFS: readonly Motif[] = [
  ...Object.values(FAMILY_MOTIFS), ...Object.values(PLATFORM_MOTIFS)
]

// Builds the map from each emoji the site displays as its own to its emblem,
// keyed on each family badge `GLOSSARY_FAMILY_META` declares and on the emoji
// leading each wheel-platform label.
export function emblemsOf(
  platforms : readonly Pick<WheelPlatform, 'emoji' | 'name'>[]
): ReadonlyMap<string, Emblem> {
  return new Map([
    ...(Object.keys(GLOSSARY_FAMILY_META) as GlossaryFamily[])
      .map((family): [string, Emblem] => [GLOSSARY_FAMILY_META[family].badge, familyEmblem(family)]),
    ...platforms.map(({ emoji, name }): [string, Emblem] => [emoji, platformEmblem(name)])
  ])
}

export function familyEmblem(family: GlossaryFamily): Emblem {
  return { family, motif: FAMILY_MOTIFS[family], name: GLOSSARY_FAMILY_META[family].label }
}

export function platformEmblem(name: string): Emblem {
  return { motif: lookup(PLATFORM_MOTIFS, name, 'Platform'), name }
}
