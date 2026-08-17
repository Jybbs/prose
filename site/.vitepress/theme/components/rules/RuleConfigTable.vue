<script setup lang="ts">
import { computed } from 'vue'

import { data as configs } from '../../../lib/rules/rule-configs.data'
import { useCurrentRule }  from '../../../lib/composables/route'
import ConfigRowTable      from '../base/ConfigRowTable.vue'

const current = useCurrentRule()

const rows = computed(() => configs[current.value?.slug ?? ''] ?? [])
</script>

<template>
  <ConfigRowTable :rows="rows" />
</template>
