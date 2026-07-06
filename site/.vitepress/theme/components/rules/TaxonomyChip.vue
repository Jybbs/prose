<script setup lang="ts">
import { computed } from 'vue'

import Chip from '../base/Chip.vue'

import * as registries    from '../../../lib/shared/registries'
import { useCurrentRule } from '../../../lib/composables/route'

const props = withDefaults(defineProps<{
  axis    : 'category' | 'family'
  linked ?: boolean
  value  ?: registries.RuleCategory | registries.RuleFamily
}>(), { linked: true })

const rule    = useCurrentRule()
const value   = computed(() => props.value ?? rule.value?.[props.axis] ?? null)
const meta    = computed(() => {
  if (!value.value) return null
  return isCategory(value.value) ? registries.CATEGORY_META[value.value] : registries.FAMILY_META[value.value]
})
const variant = computed((): 'category-chip' | 'family-chip' => `${props.axis}-chip`)
const href    = computed(() => props.linked && value.value ? `/rules/${value.value}/` : undefined)

function isCategory(v: registries.RuleCategory | registries.RuleFamily): v is registries.RuleCategory {
  return v in registries.CATEGORY_META
}
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
