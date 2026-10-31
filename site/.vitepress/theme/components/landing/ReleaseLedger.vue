<script setup lang="ts">
import { computed } from 'vue'

import type { PyPIRelease } from '../../../lib/landing/pypi-releases.data'
import { externalAttrs }    from '../../../lib/shared/links'

const props = defineProps<{ releases: readonly PyPIRelease[] }>()

const groupedByYear = computed(() =>
  Array.from(Map.groupBy(props.releases, r => r.year),
             ([year, items]) => ({ items, year }))
)
</script>

<template>
  <div class="landing-cta-scroll relative -ms-10 min-h-0 flex-1 self-stretch overflow-x-hidden overflow-y-auto">
    <div v-for="group in groupedByYear" :key="group.year" class="ps-14 not-first:mt-6">
      <p class="sticky top-0 z-1 bg-panel-bg px-8 pt-4 pb-2 font-mono text-kicker-sm tracking-display-wide text-ube uppercase">{{ group.year }}</p>
      <ol class="flex flex-col gap-1">
        <li v-for="r in group.items" :key="r.version">
          <a class="grid grid-cols-auto-minmax-12-1fr-auto items-baseline gap-6 rounded-sm px-8 py-4 font-mono text-text-1 transition-background-and-color hover:bg-stamp-accent-medium-bg hover:text-stamp-ink" :href="r.url" v-bind="externalAttrs(r.url)">
            <span class="font-display text-0-98rem text-stamp-ink italic">{{ r.version }}</span>
            <span class="mx-2 leader" aria-hidden="true"></span>
            <span class="text-0-66rem tracking-0-12em text-text-3">{{ r.month }}</span>
          </a>
        </li>
      </ol>
    </div>
  </div>
</template>
