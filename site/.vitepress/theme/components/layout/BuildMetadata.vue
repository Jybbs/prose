<script setup lang="ts">
import { useData }  from 'vitepress'
import { computed } from 'vue'

import { data as build }    from '../../../data/build.data'
import { data as rules }    from '../../../data/rules.data'

const { page } = useData()

const lastUpdated = computed(() => {
  const ts = page.value.lastUpdated
  return ts ? new Date(ts).toLocaleDateString('en-CA', { timeZone: 'UTC' }) : ''
})

interface Item {
  code   ?: boolean
  label   : string
  value   : string | number
}

const items = computed((): Item[] => [
  {             label: 'Version',  value: build.version       },
  { code: true, label: 'Commit',   value: build.gitSha        },
  {             label: 'Rules',    value: rules.list.length   },
  {             label: 'Fixtures', value: build.fixtureCount  },
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
