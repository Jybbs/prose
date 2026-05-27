import { defineLoader } from 'vitepress'

import { getRenderer, renderInlineField } from '../lib/markdown/renderer'
import type { PrimitiveSlug }             from '../lib/shared/registries'

export type PrimitiveLayer = 'analysis' | 'base' | 'orchestration'

interface PrimitiveEntry {
  consumedBy  : readonly string[]
  consumes    : readonly PrimitiveSlug[]
  layer       : PrimitiveLayer
  slug        : PrimitiveSlug
  summaryHtml : string
  tagline     : string
}

interface PrimitiveEntrySource {
  consumedBy : readonly string[]
  consumes   : readonly PrimitiveSlug[]
  layer      : PrimitiveLayer
  slug       : PrimitiveSlug
  summary    : string
  tagline    : string
}

interface PrimitivesCompositionData {
  byLayer : Record<PrimitiveLayer, readonly PrimitiveEntry[]>
  entries : readonly PrimitiveEntry[]
}

const SOURCES: readonly PrimitiveEntrySource[] = [
  {
    consumedBy : ['align-colons', 'align-comparisons', 'align-equals', 'align-imports', 'match-case-align'],
    consumes   : ['edit', 'source'],
    layer      : 'orchestration',
    slug       : 'aligner',
    summary    : 'Computes padding widths and emits the alignment edits every alignment rule '
               + 'consumes.',
    tagline    : 'shared alignment math'
  },
  {
    consumedBy : ['single-use-variables', 'unused-future-annotations'],
    consumes   : ['source'],
    layer      : 'analysis',
    slug       : 'binding-analysis',
    summary    : 'Per-source table indexing every write and read of every name in every lexical '
               + 'scope.',
    tagline    : 'name binding index'
  },
  {
    consumedBy : ['cli'],
    consumes   : ['source'],
    layer      : 'analysis',
    slug       : 'cache',
    summary    : 'User-level on-disk cache keyed on `(source ++ config ++ version)`, collapsing repeat runs to a stat plus a hash plus a deserialize.',
    tagline    : 'content-addressed result cache'
  },
  {
    consumedBy : ['align-colons', 'singleton-rule'],
    consumes   : ['aligner', 'source'],
    layer      : 'analysis',
    slug       : 'colon-targets',
    summary    : 'Walks the five `:` contexts in Python and emits alignment members for each.',
    tagline    : 'five-context colon walker'
  },
  {
    consumedBy : ['docstring-wrap', 'multi-line-docstrings', 'no-single-line-docstrings'],
    consumes   : ['edit', 'source'],
    layer      : 'analysis',
    slug       : 'docstring',
    summary    : 'PEP 257 walker reaching every module, class, and function docstring in source '
               + 'order.',
    tagline    : 'PEP 257 docstring walker'
  },
  {
    consumedBy : ['aligner', 'docstring', 'orderer', 'pipeline'],
    consumes   : ['source'],
    layer      : 'base',
    slug       : 'edit',
    summary    : 'The `Edit { range, content }` unit every rule emits and the pipeline applies.',
    tagline    : 'rewrite unit'
  },
  {
    consumedBy : ['alphabetize'],
    consumes   : ['edit', 'source'],
    layer      : 'orchestration',
    slug       : 'orderer',
    summary    : 'Reorders sibling AST nodes by a classifier while preserving attached comments.',
    tagline    : 'sibling reorder helper'
  },
  {
    consumedBy : ['cli'],
    consumes   : ['edit', 'rule-id', 'source', 'suppression-map'],
    layer      : 'orchestration',
    slug       : 'pipeline',
    summary    : 'Runs registered rules in deterministic order, reparses between rules, returns '
               + 'the final source.',
    tagline    : 'deterministic rule runner'
  },
  {
    consumedBy : ['pipeline', 'suppression-map'],
    consumes   : [],
    layer      : 'base',
    slug       : 'rule-id',
    summary    : 'Canonical kebab-case slug identifying each rule across CLI, config, '
               + 'suppressions, and diagnostics.',
    tagline    : 'canonical rule slug'
  },
  {
    consumedBy : [
      'aligner', 'binding-analysis', 'colon-targets', 'docstring', 'edit', 'orderer',
      'pipeline', 'suppression-map', 'walker'
    ],
    consumes   : [],
    layer      : 'base',
    slug       : 'source',
    summary    : 'Owned wrapper bundling the original text, AST, tokens, line index, and '
               + 'supporting tables.',
    tagline    : 'parsed-text wrapper'
  },
  {
    consumedBy : ['pipeline'],
    consumes   : ['rule-id', 'source'],
    layer      : 'analysis',
    slug       : 'suppression-map',
    summary    : 'Per-source index of `# fmt: off` / `# fmt: skip` / `# prose: ignore[...]` '
               + 'directives.',
    tagline    : 'directive index'
  },
  {
    consumedBy : ['cli'],
    consumes   : ['source'],
    layer      : 'analysis',
    slug       : 'walker',
    summary    : 'Ignore-aware filesystem walker yielding every `.py` / `.pyi` / `.pyw` file '
               + 'under given paths.',
    tagline    : 'ignore-aware path walker'
  }
]

declare const data: PrimitivesCompositionData
export { data }

export default defineLoader({
  watch: [],
  async load(): Promise<PrimitivesCompositionData> {
    const md      = await getRenderer()
    const entries = renderInlineField(md, SOURCES, 'summary')
    type ByLayer  = Record<PrimitiveLayer, readonly PrimitiveEntry[]>
    const byLayer = Object.groupBy(entries, e => e.layer) as ByLayer
    return { byLayer, entries }
  }
})
