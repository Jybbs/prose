<script setup lang="ts">
import { computed } from 'vue'

import { data, type ConfigKeys } from '../../../lib/reference/config-keys.data'
import ConfigRowTable            from '../base/ConfigRowTable.vue'

const props = defineProps<{ section: keyof ConfigKeys }>()

const rows = computed(() => data[props.section])
</script>

<template>
  <ConfigRowTable :rows="rows" />
</template>
