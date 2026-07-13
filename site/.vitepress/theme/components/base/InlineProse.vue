<script setup lang="ts">
import GlossaryTerm   from '../glossary/GlossaryTerm.vue'
import InlineRuleLink from '../rules/InlineRuleLink.vue'

import { BODY_LINK_CLASSES } from '../../../lib/shared/constants'

import type { InlineNode } from '../../../lib/markdown/inline-nodes'

defineProps<{ nodes: readonly InlineNode[] }>()
</script>

<template>
  <template v-for="(node, index) in nodes" :key="index">
    <GlossaryTerm v-if="node.kind === 'term'" :slug="node.slug">{{ node.text }}</GlossaryTerm>
    <InlineRuleLink v-else-if="node.kind === 'rule'" :slug="node.slug" />
    <a
      v-else-if="node.kind === 'primitive'"
      :class="BODY_LINK_CLASSES"
      :href="`/primitives/${node.slug}`"
    ><strong><code>{{ node.display }}</code></strong></a>
    <code v-else-if="node.kind === 'code'">{{ node.text }}</code>
    <component :is="node.tag" v-else-if="node.kind === 'el'" v-bind="node.attrs">
      <InlineProse :nodes="node.children" />
    </component>
    <template v-else>{{ node.text }}</template>
  </template>
</template>
