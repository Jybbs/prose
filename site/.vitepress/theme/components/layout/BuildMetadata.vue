<script setup lang="ts">
import { useData }  from 'vitepress'
import { computed } from 'vue'

import { data as build } from '../../../lib/shared/build.data'
import { data as rules } from '../../../lib/rules/rules.data'

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
  <section aria-label="Build" class="mx-auto mt-24 flex max-w-1120 flex-wrap justify-center gap-x-52 gap-y-16 border-t-1 border-t-divider px-18 pt-18 pb-14 font-mono tracking-folio">
    <span v-for="item in items" :key="item.label" class="flex flex-col items-center gap-1">
      <span class="text-md leading-none text-text-1">
        <code v-if="item.code">{{ item.value }}</code>
        <template v-else>{{ item.value }}</template>
      </span>
      <span class="text-meta tracking-0-12em text-text-3 uppercase">{{ item.label }}</span>
    </span>
  </section>
</template>
