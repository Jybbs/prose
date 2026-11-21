import { defineLoader } from 'vitepress'

import { getRenderer, inlineNodeField }       from '../markdown/renderer.ts'
import type { InlineNode }                    from '../markdown/inline-nodes.ts'
import { DIRECTIVES }                         from './directives.ts'
import { type DirectivePart, directiveParts } from './directive-parts.ts'
import type { ScopeKey }                      from './scopes.ts'

export interface Directive {
  effectNodes : InlineNode[]
  example     : string
  form        : string
  id          : string
  pairId     ?: string
  pairRole   ?: 'closes' | 'opens'
  parts       : DirectivePart[]
  scope       : ScopeKey
}

declare const data: readonly Directive[]
export { data }

export default defineLoader({
  watch: [],
  async load(): Promise<readonly Directive[]> {
    const md = await getRenderer()
    return inlineNodeField(md, DIRECTIVES, 'effect')
      .map(source => ({ ...source, parts: directiveParts(source.form) }))
  }
})
