import { defineLoader } from 'vitepress'

import { REPO_URL }                       from '../lib/constants'
import { siteDir }                        from '../lib/paths'
import { getRenderer, renderInlineField } from '../lib/render-markdown'

const root = siteDir(import.meta.url)

export interface Action {
  href  : string
  text  : string
  theme : 'brand' | 'alt'
}

export interface Link {
  href : string
  text : string
}

export interface Feature {
  bodyHtml : string
  cta      : string
  icon     : string
  link     : string
  number   : string
  title    : string
}

export interface Step {
  bodyHtml : string
  code     : string
  number   : string
  title    : string
}

export interface LandingData {
  cta      : { links: readonly Link[] }
  features : readonly Feature[]
  hero     : { actions: readonly Action[] }
  workflow : readonly Step[]
}

declare const data: LandingData
export { data }

const ACTIONS: readonly Action[] = [
  { href: '/guide/installation', text: 'Get Started', theme: 'brand' },
  { href: REPO_URL,              text: 'GitHub',      theme: 'alt'   },
  { href: '/rules/',             text: 'Rules',       theme: 'alt'   }
]

const CTA_LINKS: readonly Link[] = [
  { href: '/guide/configuration', text: 'Configuration' },
  { href: '/guide/installation',  text: 'Installation'  },
  { href: '/primitives/source',   text: 'Primitives'    },
  { href: '/rules/',              text: 'Rules catalog' }
]

interface FeatureSource {
  body   : string
  cta    : string
  icon   : string
  link   : string
  number : string
  title  : string
}

const FEATURE_SOURCES: readonly FeatureSource[] = [
  {
    body   : 'Equals signs, colons, the `import` keyword, and match arrows line up across consecutive lines. The eye drops down the column.',
    cta    : 'align-equals',
    icon   : '🪜',
    link   : '/rules/align-equals',
    number : '01',
    title  : 'Alignment'
  },
  {
    body   : 'Dictionaries, lists, and sets expand to one entry per line. Multi-line collections drop their trailing comma. Single-entry contexts skip padding.',
    cta    : 'collection-layout',
    icon   : '🪶',
    link   : '/rules/collection-layout',
    number : '02',
    title  : 'Layout'
  },
  {
    body   : 'Legacy union syntax, loose constants, step-narration comments, and single-use bindings surface as lint diagnostics, never rewrites.',
    cta    : 'single-use-variables',
    icon   : '🧶',
    link   : '/rules/single-use-variables',
    number : '03',
    title  : 'Lint'
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
    const md = await getRenderer(root)
    return {
      cta     : { links: CTA_LINKS },
      features: renderInlineField(md, FEATURE_SOURCES, 'body'),
      hero    : { actions: ACTIONS },
      workflow: renderInlineField(md, STEP_SOURCES, 'body')
    }
  }
})
