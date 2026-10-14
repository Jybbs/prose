<script setup lang="ts">
import { computed } from 'vue'

import type { Emblem }    from '../../../lib/shared/emblems'
import { nibId, nibSize } from '../../../lib/shared/nib'

const props = defineProps<{
  emblem  : Emblem
  label  ?: string
  size    : number
}>()

const href = computed(() => `#${nibId(props.emblem.motif, nibSize(props.size))}`)
</script>

<template>
  <svg
    class="nib-badge"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    :role="label ? 'img' : undefined"
    :aria-label="label"
    :aria-hidden="label ? undefined : 'true'"
  ><use :href="href" /></svg>
</template>
