<script setup lang="ts">
import { computed } from 'vue'

import { data as configKeys, type ConfigKeys } from '../../../lib/reference/config-keys.data'
import { data as ruleConfigs }                 from '../../../lib/rules/rule-configs.data'

const props = withDefaults(
  defineProps<{ facet: string, rule?: string, section?: keyof ConfigKeys }>(),
  { rule: undefined, section: 'top' }
)

const value = computed(() => {
  const rows = props.rule === undefined ? configKeys[props.section] : ruleConfigs[props.rule]
  const row  = rows?.find(r => r.key === props.facet)
  if (row === undefined) {
    throw new Error(`ConfigDefault: no key ${props.facet} under ${props.rule ?? props.section}`)
  }
  return row.default
})
</script>

<template>
  <code>{{ value }}</code>
</template>
