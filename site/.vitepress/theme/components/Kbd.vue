<script setup lang="ts">
defineProps<{ keys: string }>()
</script>

<template>
  <span class="kbd-shortcut">
    <kbd v-for="(k, idx) in keys.split('+')" :key="idx">{{ k.trim() }}</kbd>
  </span>
</template>
