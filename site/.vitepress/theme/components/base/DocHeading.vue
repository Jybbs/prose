<script setup lang="ts">
import PermalinkAnchor from './PermalinkAnchor.vue'

defineProps<{ id: string, title: string }>()
</script>

<template>
  <h2 :id="id" tabindex="-1">
    {{ title }}
    <PermalinkAnchor :anchor="id" :label="title" />
  </h2>
</template>
