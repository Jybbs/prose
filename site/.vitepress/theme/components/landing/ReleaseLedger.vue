<script setup lang="ts">
import { computed } from 'vue'

import type { PyPIRelease } from '../../../data/pypi-releases.data'
import { externalAttrs }    from '../../../lib/shared/links'

const props = defineProps<{ releases: readonly PyPIRelease[] }>()

const groupedByYear = computed(() =>
  Array.from(Map.groupBy(props.releases, r => r.year),
             ([year, items]) => ({ items, year }))
)
</script>

<template>
  <div class="landing-cta-scroll">
    <div v-for="group in groupedByYear" :key="group.year" class="landing-cta-group">
      <p class="landing-cta-year">{{ group.year }}</p>
      <ol class="landing-cta-ledger">
        <li v-for="r in group.items" :key="r.version" class="landing-cta-ledger-row">
          <a :href="r.url" v-bind="externalAttrs(r.url)">
            <span class="landing-cta-ledger-version">{{ r.version }}</span>
            <span class="landing-cta-ledger-leader" aria-hidden="true"></span>
            <span class="landing-cta-ledger-month">{{ r.month }}</span>
          </a>
        </li>
      </ol>
    </div>
  </div>
</template>
