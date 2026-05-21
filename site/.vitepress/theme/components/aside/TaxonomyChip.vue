<script setup lang="ts">
import { computed } from 'vue'

import Chip from '../base/Chip.vue'

import { CATEGORY_META, FAMILY_META, type RuleCategory, type RuleFamily } from '../../../lib/shared/registries'
import { useCurrentRule }                                                  from '../../../lib/shared/route'

const props = withDefaults(defineProps<{
  axis    : 'category' | 'family'
  linked ?: boolean
  value  ?: RuleCategory | RuleFamily
}>(), { linked: true })

const REGISTRY = { category: CATEGORY_META, family: FAMILY_META } as const

const rule    = useCurrentRule()
const value   = computed(() => props.value ?? rule.value?.[props.axis] ?? null)
const meta    = computed(() => value.value ? REGISTRY[props.axis][value.value as never] as { badge: string; label: string } : null)
const variant = computed(() => `${props.axis}-chip` as 'category-chip' | 'family-chip')
const href    = computed(() => props.linked && value.value ? `/rules/${value.value}/` : undefined)
</script>

<template>
  <Chip
    v-if="meta && value"
    :variant="variant"
    :[axis]="value"
    :href="href"
  >
    <span :class="`${axis}-chip-badge`" aria-hidden="true">{{ meta.badge }}</span>
    <span :class="`${axis}-chip-label`">{{ meta.label }}</span>
  </Chip>
</template>
