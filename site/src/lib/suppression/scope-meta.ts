export const SCOPES = ['block', 'dict', 'file', 'line'] as const
export type ScopeKey = (typeof SCOPES)[number]

export const SCOPE_META: Record<ScopeKey, { anchor: string, label: string, pip: string }> = {
  block : { anchor : 'block-markers',                   label : 'Block',        pip : 'B' },
  dict  : { anchor : 'dict-literal-order-preservation', label : 'Dict literal', pip : 'D' },
  file  : { anchor : 'file-level-suppression',          label : 'File',         pip : 'F' },
  line  : { anchor : 'line-markers',                    label : 'Line',         pip : 'L' }
}

export const SCOPE_ORDER: ScopeKey[] = ['file', 'block', 'line', 'dict']

export const directiveHref = (scope: ScopeKey): string =>
  `/reference/suppression-directives#${SCOPE_META[scope].anchor}`

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  describe('scope metadata', () => {
    test('describes every scope key', () => {
      expect(Object.keys(SCOPE_META).sort()).toEqual([...SCOPES].sort())
    })

    test('orders the same set it describes', () => {
      expect([...SCOPE_ORDER].sort()).toEqual([...SCOPES].sort())
    })

    test.each(SCOPES.map(key => ({ name: key, meta: SCOPE_META[key] })))(
      'gives the $name scope an anchor, label and single-letter pip',
      ({ meta }) => {
        expect(meta.anchor).toMatch(/^[a-z-]+$/)
        expect(meta.label.length).toBeGreaterThan(0)
        expect(meta.pip).toMatch(/^[A-Z]$/)
      }
    )
  })
}
