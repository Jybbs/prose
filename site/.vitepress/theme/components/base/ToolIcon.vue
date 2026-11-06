<script setup lang="ts">
import { computed } from 'vue'

import { data as tools } from '../../../lib/shared/tools.data'
import { lookup }        from '../../../lib/shared/lookup'

const props = defineProps<{
  slug : string
}>()

const icon = computed(() => lookup(tools.entries, props.slug, 'ToolIcon').icon)
</script>

<template>
  <svg
    :viewBox="icon.viewBox"
    aria-hidden="true"
    v-html="icon.body"
  />
</template>
