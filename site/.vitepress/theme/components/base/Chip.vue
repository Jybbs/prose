<script setup lang="ts">
import { computed } from 'vue'

import { externalAttrs } from '../../../lib/shared/links'

type Variant = 'family-chip' | 'rule-chip'

const VARIANT_CLASSES: Record<Variant, string> = {
  'family-chip' : 'chip family-chip',
  'rule-chip'   : 'chip rule-chip'
}

const props = defineProps<{
  family ?: string
  href   ?: string
  title  ?: string
  variant : Variant
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
    :class="VARIANT_CLASSES[variant]"
    v-bind="linkAttrs"
  ><slot /></component>
</template>
