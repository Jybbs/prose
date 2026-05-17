<script setup lang="ts">
import { computed } from 'vue'
import { useData }  from 'vitepress'

import { data as rules } from '../../data/rules.data'
import { data as build } from '../../data/build.data'

const { page } = useData()

const lastUpdated = computed(() => {
  const ts = page.value.lastUpdated
  return ts ? new Date(ts).toISOString().slice(0, 10) : ''
})

interface Item {
  code   ?: boolean
  label   : string
  value   : string | number
}

const items = computed((): Item[] => [
  { label: 'Version',                  value: build.version },
  { label: 'Commit',  code: true,      value: build.gitSha },
  { label: 'Rules',                    value: rules.length },
  { label: 'Fixtures',                 value: build.fixtureCount },
  ...(lastUpdated.value ? [{ label: 'Updated', value: lastUpdated.value }] : [])
])
</script>

<template>
  <footer class="build-metadata">
    <span v-for="item in items" :key="item.label" class="build-metadata-item">
      <span class="build-metadata-label">{{ item.label }}</span>
      <span class="build-metadata-value">
        <code v-if="item.code">{{ item.value }}</code>
        <template v-else>{{ item.value }}</template>
      </span>
    </span>
  </footer>
</template>
