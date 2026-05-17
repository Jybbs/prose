<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'

const props = defineProps<{
  storageKey?: string
  versions  : string[]
}>()

const STORAGE_KEY = props.storageKey ?? 'prose-target-version'
const current    = ref(props.versions[0])

onMounted(() => {
  const stored = typeof localStorage !== 'undefined' ? localStorage.getItem(STORAGE_KEY) : null
  if (stored && props.versions.includes(stored)) current.value = stored
})

watch(current, value => {
  if (typeof localStorage !== 'undefined') localStorage.setItem(STORAGE_KEY, value)
})
</script>

<template>
  <div class="version-selector">
    <span class="version-selector-label">Target version</span>
    <select v-model="current">
      <option v-for="v in versions" :key="v" :value="v">Python {{ v }}</option>
    </select>
  </div>
  <template v-for="v in versions" :key="v">
    <div v-show="current === v">
      <slot :name="v" />
    </div>
  </template>
</template>
