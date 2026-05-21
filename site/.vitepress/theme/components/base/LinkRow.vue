<script setup lang="ts">
import { externalAttrs } from '../../../lib/shared/links'

export interface LinkRowItem {
  className ?: string
  href       : string
  text       : string
}

defineProps<{
  items   : readonly LinkRowItem[]
  variant : string
}>()
</script>

<template>
  <div :class="variant">
    <a
      v-for="item in items"
      :key="item.href"
      :href="item.href"
      :class="item.className"
      v-bind="externalAttrs(item.href)"
    >{{ item.text }}</a>
  </div>
</template>
