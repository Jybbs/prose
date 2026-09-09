<script setup lang="ts">
defineProps<{ anchor: string, label: string }>()
</script>

<template>
  <a class="header-anchor" :href="`#${anchor}`" :aria-label="`Permalink to “${label}”`">​</a>
</template>
