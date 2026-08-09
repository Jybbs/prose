<script setup lang="ts">
import { useAriaHidden }     from '../../../lib/composables/use-aria-hidden'
import type { InlineNode }   from '../../../lib/markdown/inline-nodes'
import { BODY_LINK_CLASSES } from '../../../lib/shared/constants'
import GlossaryTerm          from '../glossary/GlossaryTerm.vue'
import InlineRuleLink        from '../rules/InlineRuleLink.vue'

defineProps<{ nodes: readonly InlineNode[] }>()

const ariaHidden = useAriaHidden()
</script>

<template>
  <template v-for="(node, index) in nodes" :key="index">
    <GlossaryTerm v-if="node.kind === 'term'" :slug="node.slug">{{ node.text }}</GlossaryTerm>
    <InlineRuleLink v-else-if="node.kind === 'rule'" :slug="node.slug" />
    <a
      v-else-if="node.kind === 'primitive'"
      :class="BODY_LINK_CLASSES"
      :href="`/primitives/${node.slug}`"
      :tabindex="ariaHidden ? -1 : undefined"
    ><strong><code>{{ node.display }}</code></strong></a>
    <code v-else-if="node.kind === 'code'">{{ node.text }}</code>
    <component
      :is="node.tag"
      v-else-if="node.kind === 'el'"
      v-bind="node.attrs"
      :tabindex="node.tag === 'a' && ariaHidden ? -1 : node.attrs.tabindex"
    >
      <InlineProse :nodes="node.children" />
    </component>
    <template v-else>{{ node.text }}</template>
  </template>
</template>
