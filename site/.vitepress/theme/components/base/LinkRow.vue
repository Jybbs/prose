<script setup lang="ts">
export interface LinkRowItem {
  className ?: string
  href       : string
  text       : string
}

defineProps<{
  items   : readonly LinkRowItem[]
  variant : string
}>()

function isExternal(href: string): boolean {
  return href.startsWith('http')
}
</script>

<template>
  <div :class="variant">
    <a
      v-for="item in items"
      :key="item.href"
      :href="item.href"
      :class="item.className"
      :target="isExternal(item.href) ? '_blank' : undefined"
      :rel="isExternal(item.href) ? 'noopener' : undefined"
    >{{ item.text }}</a>
  </div>
</template>
