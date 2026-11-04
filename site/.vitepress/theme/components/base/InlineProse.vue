<script setup lang="ts">
import { useAriaHidden, useHiddenTabindex } from '../../../lib/composables/use-aria-hidden'
import { codeParts }                        from '../../../lib/markdown/code-operators'
import type { InlineNode }                  from '../../../lib/markdown/inline-nodes'
import { BODY_LINK_CLASS }                  from '../../../lib/shared/constants'
import { primitiveRoute }                   from '../../../lib/shared/routes'
import GlossaryTerm                         from '../glossary/GlossaryTerm.vue'
import InlineRuleLink                       from '../rules/InlineRuleLink.vue'

defineProps<{
  linked ?: boolean
  nodes   : readonly InlineNode[]
}>()

const ariaHidden = useAriaHidden()
const tabindex   = useHiddenTabindex()
</script>

<template>
  <template v-for="(node, index) in nodes" :key="index">
    <GlossaryTerm v-if="node.kind === 'term'" :slug="node.slug">{{ node.text }}</GlossaryTerm>
    <InlineRuleLink v-else-if="node.kind === 'rule'" :slug="node.slug" />
    <a
      v-else-if="node.kind === 'primitive'"
      :class="BODY_LINK_CLASS"
      :href="primitiveRoute(node.slug)"
      :tabindex="tabindex"
    ><strong><code>{{ node.display }}</code></strong></a>
    <code v-else-if="node.kind === 'code'"><template v-for="(part, at) in linked ? [{ op: false, text: node.text }] : codeParts(node.text)" :key="at"><span v-if="part.op" class="code-op">{{ part.text }}</span><template v-else>{{ part.text }}</template></template></code>
    <component
      :is="node.tag"
      v-else-if="node.kind === 'el'"
      v-bind="node.attrs"
      :tabindex="node.tag === 'a' && ariaHidden ? -1 : node.attrs.tabindex"
    >
      <InlineProse :linked="linked || node.tag === 'a'" :nodes="node.children" />
    </component>
    <template v-else>{{ node.text }}</template>
  </template>
</template>
