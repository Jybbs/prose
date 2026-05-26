<script setup lang="ts">
import { computed } from 'vue'

import { externalAttrs } from '../../../lib/shared/links'

const props = defineProps<{
  category ?: string
  family   ?: string
  href     ?: string
  title    ?: string
  variant   : 'category-chip' | 'family-chip' | 'rule-chip' | 'rule-chip pipeline-order-undocumented' | 'star-count'
}>()

const tag       = computed(() => (props.href ? 'a' : 'div'))
const linkAttrs = computed(() => externalAttrs(props.href))
</script>

<template>
  <component
    :is="tag"
    :href="href"
    :title="title"
    :data-category="category"
    :data-family="family"
    :class="variant"
    v-bind="linkAttrs"
  ><slot /></component>
</template>
