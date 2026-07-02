export const SCOPES = ['block', 'dict', 'file', 'line'] as const
export type ScopeKey = (typeof SCOPES)[number]

export const SCOPE_META: Record<ScopeKey, { anchor: string; label: string; pip: string }> = {
  block : { anchor : 'block-markers',                   label : 'Block',        pip : 'B' },
  dict  : { anchor : 'dict-literal-order-preservation', label : 'Dict literal', pip : 'D' },
  file  : { anchor : 'file-level-suppression',          label : 'File',         pip : 'F' },
  line  : { anchor : 'line-markers',                    label : 'Line',         pip : 'L' }
}

export const SCOPE_ORDER: ScopeKey[] = ['file', 'block', 'line', 'dict']
