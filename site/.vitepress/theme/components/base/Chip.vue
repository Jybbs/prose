<script setup lang="ts">
import { computed } from 'vue'

import { externalAttrs } from '../../../lib/shared/links'

const props = defineProps<{
  family ?: string
  href   ?: string
  title  ?: string
  variant : 'family-chip' | 'rule-chip' | 'star-count'
}>()

const tag       = computed(() => (props.href ? 'a' : 'span'))
const linkAttrs = computed(() => externalAttrs(props.href))
</script>

<template>
  <component
    :is="tag"
    :href="href"
    :title="title"
    :data-family="family"
    :class="variant"
    v-bind="linkAttrs"
  ><slot /></component>
</template>
