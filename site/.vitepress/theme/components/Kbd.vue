<script setup lang="ts">
defineProps<{ keys: string }>()
</script>

<template>
  <span class="kbd-shortcut">
    <kbd v-for="(k, idx) in keys.split('+')" :key="idx">{{ k.trim() }}</kbd>
  </span>
</template>

<style scoped>
.kbd-shortcut {
  display    : inline-flex;
  align-items: center;
  gap        : 4px;
}

.kbd-shortcut kbd + kbd {
  margin-left: 0;
}
</style>
