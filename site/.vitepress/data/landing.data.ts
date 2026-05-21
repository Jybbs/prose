import { defineLoader } from 'vitepress'

import { getRenderer, renderInlineField } from '../lib/markdown/renderer'
import type { RuleFamily }                from '../lib/shared/registries'

export interface Link {
  href : string
  text : string
}

export interface Step {
  bodyHtml : string
  code     : string
  number   : string
  title    : string
}

export interface Surface {
  bodyHtml : string
  family   : RuleFamily
  icon     : string
  number   : string
}

export interface LandingData {
  cta      : { links: readonly Link[] }
  surfaces : readonly Surface[]
  workflow : readonly Step[]
}

declare const data: LandingData
export { data }

const CTA_LINKS: readonly Link[] = [
  { href: '/guide/installation',      text: 'Installation'  },
  { href: '/primitives/source',       text: 'Primitives'    },
  { href: '/reference/configuration', text: 'Configuration' },
  { href: '/rules/',                  text: 'Rules catalog' }
]

interface SurfaceSource {
  body   : string
  family : RuleFamily
  icon   : string
  number : string
}

const SURFACE_SOURCES: readonly SurfaceSource[] = [
  {
    body   : 'Equals signs, colons, the `import` keyword, and match arrows line up across consecutive lines. The eye drops down the column.',
    family : 'alignment',
    icon   : '🪜',
    number : '01'
  },
  {
    body   : 'Sibling entries sort into a predictable order. Imports, dictionary keys, and set members read top-to-bottom by name, so a reader looking for an entry already knows where it sits.',
    family : 'ordering',
    icon   : '🪉',
    number : '02'
  },
  {
    body   : 'Dictionaries, lists, and sets expand to one entry per line. Multi-line collections drop their trailing comma, blank lines snap to canonical counts, and singletons collapse to their natural form.',
    family : 'formatting',
    icon   : '🪶',
    number : '03'
  },
  {
    body   : 'Docstrings join the same legibility discipline as code. Wrap to the project line length, keep single-line shapes single-line, multi-line shapes multi-line, and quote style consistent throughout.',
    family : 'docs',
    icon   : '📰',
    number : '04'
  },
  {
    body   : 'Legacy union syntax, loose constants, step-narration comments, bare-import patterns, and single-use bindings surface as diagnostics. The formatter never rewrites these, because the fix belongs to the reader.',
    family : 'lint',
    icon   : '🧶',
    number : '05'
  }
]

interface StepSource {
  body   : string
  code   : string
  number : string
  title  : string
}

const STEP_SOURCES: readonly StepSource[] = [
  {
    body   : 'Fetch the wheel and expose the `prose` binary.',
    code   : 'uv tool install prose-formatter',
    number : '01',
    title  : 'Install'
  },
  {
    body   : 'Drop a `[tool.prose]` table into `pyproject.toml`. The defaults already work.',
    code   : '[tool.prose]\ntarget-version = "3.13"',
    number : '02',
    title  : 'Configure'
  },
  {
    body   : 'Rewrite in place, or check without writing.',
    code   : 'prose format path/\nprose check path/',
    number : '03',
    title  : 'Run'
  },
  {
    body   : 'Pair with Ruff as the token-level upstream pass.',
    code   : 'ruff format && prose format',
    number : '04',
    title  : 'Compose'
  }
]

export default defineLoader({
  watch: [],
  async load(): Promise<LandingData> {
    const md = await getRenderer()
    return {
      cta     : { links: CTA_LINKS },
      surfaces: renderInlineField(md, SURFACE_SOURCES, 'body'),
      workflow: renderInlineField(md, STEP_SOURCES, 'body')
    }
  }
})
