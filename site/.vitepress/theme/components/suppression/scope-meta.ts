export type ScopeKey = 'block' | 'dict' | 'file' | 'line'

export const SCOPE_META: Record<ScopeKey, { label: string; pip: string }> = {
  block : { label : 'Block',        pip : 'B' },
  dict  : { label : 'Dict literal', pip : 'D' },
  file  : { label : 'File',         pip : 'F' },
  line  : { label : 'Line',         pip : 'L' }
}

export const SCOPE_ORDER: ScopeKey[] = ['file', 'block', 'line', 'dict']
