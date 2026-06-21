import type { ScopeKey } from './scope-meta'

interface Decision {
  directive : string
  id        : string
  scope     : ScopeKey
}

export const DECISIONS: Decision[] = [
  { directive : '# prose: off',            id : 'file-off',        scope : 'file'  },
  { directive : '# fmt: off … # fmt: on',  id : 'block-fmt',       scope : 'block' },
  { directive : '# fmt: skip',             id : 'line-skip',       scope : 'line'  },
  { directive : '# prose: skip[<rule>]',   id : 'line-skip-rules', scope : 'line'  },
  { directive : '# prose: ignore[<rule>]', id : 'line-ignore',     scope : 'line'  },
  { directive : '# prose: keep',           id : 'dict-keep',       scope : 'dict'  }
]
