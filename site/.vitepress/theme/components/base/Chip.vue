<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  category ?: string
  domain   ?: string
  href     ?: string
  title    ?: string
  variant   : 'category-chip' | 'domain-chip' | 'rule-chip' | 'star-count'
}>()

const isExternal = computed(() => props.href?.startsWith('http') ?? false)
const tag        = computed(() => (props.href ? 'a' : 'div'))
</script>

<template>
  <component
    :is="tag"
    :href="href"
    :title="title"
    :data-category="category"
    :data-domain="domain"
    :target="isExternal ? '_blank' : undefined"
    :rel="isExternal ? 'noopener' : undefined"
    :class="variant"
  ><slot /></component>
</template>
