import { defineComponent, Fragment, h } from 'vue'
import type { PropType }                from 'vue'

import { useAriaHidden, useHiddenTabindex } from '../../../lib/composables/use-aria-hidden'
import { codeParts }                        from '../../../lib/markdown/code-operators'
import type { InlineNode }                  from '../../../lib/markdown/inline-nodes'
import { BODY_LINK_CLASS }                  from '../../../lib/shared/constants'
import { primitiveRoute }                   from '../../../lib/shared/routes'
import { GlossaryTerm }                     from '../glossary/GlossaryTerm'
import { InlineRuleLink }                   from '../rules/InlineRuleLink'

export const InlineProse = defineComponent({
  name  : 'InlineProse',
  props : {
    linked : Boolean,
    nodes  : { required: true, type: Array as PropType<readonly InlineNode[]> }
  },
  setup(props) {
    const ariaHidden = useAriaHidden()
    const tabindex   = useHiddenTabindex()

    return () => (
      props.nodes.map((node, index) => (
        <Fragment key={index}>
          {node.kind === 'term'
            ? (
              <GlossaryTerm slug={node.slug}>
                {{ default: () => node.text }}
              </GlossaryTerm>
            )
            : node.kind === 'rule'
            ? <InlineRuleLink slug={node.slug} />
            : node.kind === 'primitive'
            ? (
              <a class={BODY_LINK_CLASS} href={primitiveRoute(node.slug)} tabindex={tabindex.value}>
                <strong>
                  <code>{node.display}</code>
                </strong>
              </a>
            )
            : node.kind === 'code'
            ? (
              <code>
                {(props.linked ? [{ op: false, text: node.text }] : codeParts(node.text)).map((part, at) => (
                  <Fragment key={at}>
                    {part.op
                      ? <span class="code-op">{part.text}</span>
                      : part.text}
                  </Fragment>
                ))}
              </code>
            )
            : node.kind === 'el'
            ? h(
              node.tag,
              { ...node.attrs, tabindex: node.tag === 'a' && ariaHidden.value ? -1 : node.attrs.tabindex },
              [<InlineProse linked={props.linked || node.tag === 'a'} nodes={node.children} />]
            )
            : node.text}
        </Fragment>
      ))
    )
  }
})
