<script setup lang="ts">
import { computed } from 'vue'

import Chip from '../base/Chip.vue'

import { CATEGORY_META, DOMAIN_META, type RuleCategory, type RuleDomain } from '../../../lib/shared/registries'
import { useCurrentRule }                                                  from '../../../lib/shared/route'

const props = withDefaults(defineProps<{
  axis    : 'category' | 'domain'
  linked ?: boolean
  value  ?: RuleCategory | RuleDomain
}>(), { linked: true })

const REGISTRY = { category: CATEGORY_META, domain: DOMAIN_META } as const

const rule    = useCurrentRule()
const value   = computed(() => props.value ?? rule.value?.[props.axis] ?? null)
const meta    = computed(() => value.value ? REGISTRY[props.axis][value.value as never] as { badge: string; label: string } : null)
const variant = computed(() => `${props.axis}-chip` as 'category-chip' | 'domain-chip')
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
