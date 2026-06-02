import { defineLoader } from 'vitepress'

import { getRenderer, renderFencedHtml, renderInlineField } from '../lib/markdown/renderer'
import type { RuleFamily }                                  from '../lib/shared/registries'

export interface Step {
  bodyHtml : string
  codeHtml : string
  language : string
  number   : string
  title    : string
}

export interface Surface {
  bodyHtml : string
  family   : RuleFamily
  icon     : string
  number   : string
}

interface LandingData {
  surfaces : readonly Surface[]
  workflow : readonly Step[]
}

declare const data: LandingData
export { data }

type SurfaceSource = Omit<Surface, 'bodyHtml'> & { body: string }

const SURFACE_SOURCES: readonly SurfaceSource[] = [
  {
    body   : 'Equals signs, colons, the `import` keyword, and match arrows line up across '
           + 'consecutive lines. The eye drops down the column.',
    family : 'alignment',
    icon   : '🪜',
    number : '01'
  },
  {
    body   : 'Sibling entries sort into a predictable order. Imports, dictionary keys, and '
           + 'set members read top-to-bottom by name, so a reader looking for an entry '
           + 'already knows where it sits.',
    family : 'ordering',
    icon   : '🪉',
    number : '02'
  },
  {
    body   : 'Dictionaries, lists, and sets expand to one entry per line. Multi-line '
           + 'collections drop their trailing comma, blank lines snap to canonical counts, '
           + 'and singletons collapse to their natural form.',
    family : 'formatting',
    icon   : '🪶',
    number : '03'
  },
  {
    body   : 'Docstrings join the same legibility discipline as code. Wrap to the project '
           + 'line length, keep single-line shapes single-line, multi-line shapes '
           + 'multi-line, and quote style consistent throughout.',
    family : 'docs',
    icon   : '📰',
    number : '04'
  },
  {
    body   : 'Legacy union syntax, reassigned constants, step-narration comments, bare-import '
           + 'patterns, and single-use bindings surface as diagnostics. The formatter never '
           + 'rewrites these, because the fix belongs to the reader.',
    family : 'lint',
    icon   : '🧶',
    number : '05'
  }
]

type StepSource = Omit<Step, 'bodyHtml' | 'codeHtml'> & { body: string; code: string }

const STEP_SOURCES: readonly StepSource[] = [
  {
    body     : 'Fetch the wheel and expose the `prose` binary.',
    code     : 'uv tool install prose-formatter',
    language : 'bash',
    number   : '01',
    title    : 'Install'
  },
  {
    body     : 'Drop a `prose.toml` at the project root, or a `[tool.prose]` table in `pyproject.toml`. The defaults already work.',
    code     : 'target-version = "3.13"',
    language : 'toml',
    number   : '02',
    title    : 'Configure'
  },
  {
    body     : 'Rewrite in place, or check without writing.',
    code     : 'prose format path/\nprose check path/',
    language : 'bash',
    number   : '03',
    title    : 'Run'
  },
  {
    body     : 'Optionally pair with Ruff for the token-level surface *Prose* doesn\'t touch.',
    code     : 'ruff format && prose format',
    language : 'bash',
    number   : '04',
    title    : 'Compose'
  }
]

export default defineLoader({
  watch: [],
  async load(): Promise<LandingData> {
    const md = await getRenderer()
    return {
      surfaces : renderInlineField(md, SURFACE_SOURCES, 'body'),
      workflow : STEP_SOURCES.map(src => ({
        bodyHtml : md.renderInline(src.body),
        codeHtml : renderFencedHtml(md, src.code, src.language),
        language : src.language,
        number   : src.number,
        title    : src.title
      }))
    }
  }
})
