import type { ScopeKey } from './scope-meta'

interface Decision {
  directive : string
  href      : string
  id        : string
  scope     : ScopeKey
}

const DIRECTIVES_PAGE = '/reference/suppression-directives'

export const DECISIONS: Decision[] = [
  { directive : '# prose: off',            href : `${DIRECTIVES_PAGE}#file-level-suppression`,           id : 'file-off',        scope : 'file'  },
  { directive : '# fmt: off … # fmt: on',  href : `${DIRECTIVES_PAGE}#block-markers`,                    id : 'block-fmt',       scope : 'block' },
  { directive : '# fmt: skip',             href : `${DIRECTIVES_PAGE}#line-markers`,                     id : 'line-skip',       scope : 'line'  },
  { directive : '# prose: skip[<rule>]',   href : `${DIRECTIVES_PAGE}#line-markers`,                     id : 'line-skip-rules', scope : 'line'  },
  { directive : '# prose: ignore[<rule>]', href : `${DIRECTIVES_PAGE}#line-markers`,                     id : 'line-ignore',     scope : 'line'  },
  { directive : '# prose: keep',           href : `${DIRECTIVES_PAGE}#dict-literal-order-preservation`,  id : 'dict-keep',       scope : 'dict'  }
]
