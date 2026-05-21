import type { PrimitiveSlug } from '../../../lib/shared/registries'
import { PRIMITIVES }         from '../../../lib/shared/registries'

export type PrimitiveLayer = 'base' | 'orchestration' | 'analysis'

export interface PrimitiveEntry {
  consumedBy : readonly string[]
  consumes   : readonly PrimitiveSlug[]
  layer      : PrimitiveLayer
  slug       : PrimitiveSlug
  summary    : string
  tagline    : string
}

export const LAYER_META: Record<PrimitiveLayer, { kicker: string, label: string }> = {
  analysis      : { kicker: 'L3 · analysis',      label: 'Analysis & walkers' },
  base          : { kicker: 'L1 · base',          label: 'Base values'        },
  orchestration : { kicker: 'L2 · orchestration', label: 'Orchestration'      }
}

export const PRIMITIVE_ENTRIES: readonly PrimitiveEntry[] = [
  {
    consumedBy : ['align-equals', 'align-colons', 'align-imports', 'match-case-align'],
    consumes   : ['source', 'edit'],
    layer      : 'orchestration',
    slug       : 'aligner',
    summary    : 'Computes padding widths and emits the alignment edits every alignment rule consumes.',
    tagline    : 'shared alignment math'
  },
  {
    consumedBy : ['single-use-variables', 'unused-future-annotations'],
    consumes   : ['source'],
    layer      : 'analysis',
    slug       : 'binding-analysis',
    summary    : 'Per-source table indexing every write and read of every name in every lexical scope.',
    tagline    : 'name binding index'
  },
  {
    consumedBy : ['align-colons', 'singleton-rule'],
    consumes   : ['source', 'aligner'],
    layer      : 'analysis',
    slug       : 'colon-targets',
    summary    : 'Walks the five `:` contexts in Python and emits alignment members for each.',
    tagline    : 'five-context colon walker'
  },
  {
    consumedBy : ['docstring-wrap', 'multi-line-docstrings', 'no-single-line-docstrings'],
    consumes   : ['source', 'edit'],
    layer      : 'analysis',
    slug       : 'docstring',
    summary    : 'PEP 257 walker reaching every module, class, and function docstring in source order.',
    tagline    : 'PEP 257 docstring walker'
  },
  {
    consumedBy : ['pipeline', 'aligner', 'orderer', 'docstring'],
    consumes   : ['source'],
    layer      : 'base',
    slug       : 'edit',
    summary    : 'The `Edit { range, content }` unit every rule emits and the pipeline applies.',
    tagline    : 'rewrite unit'
  },
  {
    consumedBy : ['alphabetize'],
    consumes   : ['source', 'edit'],
    layer      : 'orchestration',
    slug       : 'orderer',
    summary    : 'Reorders sibling AST nodes by a classifier while preserving attached comments.',
    tagline    : 'sibling reorder helper'
  },
  {
    consumedBy : ['cli'],
    consumes   : ['source', 'rule-id', 'edit', 'suppression-map'],
    layer      : 'orchestration',
    slug       : 'pipeline',
    summary    : 'Runs registered rules in deterministic order, reparses between rules, returns the final source.',
    tagline    : 'deterministic rule runner'
  },
  {
    consumedBy : ['pipeline', 'suppression-map'],
    consumes   : [],
    layer      : 'base',
    slug       : 'rule-id',
    summary    : 'Canonical kebab-case slug identifying each rule across CLI, config, suppressions, and diagnostics.',
    tagline    : 'canonical rule slug'
  },
  {
    consumedBy : ['pipeline', 'aligner', 'orderer', 'binding-analysis', 'suppression-map', 'colon-targets', 'docstring', 'walker', 'edit'],
    consumes   : [],
    layer      : 'base',
    slug       : 'source',
    summary    : 'Owned wrapper bundling the original text, AST, tokens, line index, and supporting tables.',
    tagline    : 'parsed-text wrapper'
  },
  {
    consumedBy : ['pipeline'],
    consumes   : ['source', 'rule-id'],
    layer      : 'analysis',
    slug       : 'suppression-map',
    summary    : 'Per-source index of `# fmt: off` / `# fmt: skip` / `# prose: ignore[...]` directives.',
    tagline    : 'directive index'
  },
  {
    consumedBy : ['cli'],
    consumes   : ['source'],
    layer      : 'analysis',
    slug       : 'walker',
    summary    : 'Ignore-aware filesystem walker yielding every `.py` / `.pyi` / `.pyw` file under given paths.',
    tagline    : 'ignore-aware path walker'
  }
]

export function displayName(slug: PrimitiveSlug): string {
  return PRIMITIVES[slug]
}

export function entriesByLayer(layer: PrimitiveLayer): readonly PrimitiveEntry[] {
  return PRIMITIVE_ENTRIES.filter(e => e.layer === layer)
}
