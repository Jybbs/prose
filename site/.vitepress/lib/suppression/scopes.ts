export const SCOPE_ORDER = ['file', 'block', 'line', 'construct'] as const

export type ScopeKey = (typeof SCOPE_ORDER)[number]

export const SCOPE_META: Record<ScopeKey, { anchor: string, label: string, pip: string }> = {
  block     : { anchor : 'block-markers',                label : 'Block',     pip : 'B' },
  construct : { anchor : 'construct-order-preservation', label : 'Construct', pip : 'C' },
  file      : { anchor : 'file-level-suppression',       label : 'File',      pip : 'F' },
  line      : { anchor : 'line-markers',                 label : 'Line',      pip : 'L' }
}

export function directiveHref(scope: ScopeKey): string {
  return `/reference/suppression-directives#${SCOPE_META[scope].anchor}`
}

export function scopeBands<T extends { scope: ScopeKey }>(
  items: readonly T[]
): Array<{ items: T[], scope: ScopeKey }> {
  return SCOPE_ORDER.map(scope => ({
    items : items.filter(item => item.scope === scope),
    scope : scope
  }))
}
