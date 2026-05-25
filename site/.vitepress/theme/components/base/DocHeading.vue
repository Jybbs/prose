<script setup lang="ts">
defineProps<{ id: string; title: string }>()
</script>

<template>
  <h2 :id="id" tabindex="-1">
    {{ title }}
    <a class="header-anchor" :href="`#${id}`" :aria-label="`Permalink to &quot;${title}&quot;`">​</a>
  </h2>
</template>
