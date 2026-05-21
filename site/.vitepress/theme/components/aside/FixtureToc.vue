<script setup lang="ts">
import { computed } from 'vue'

import Kicker from '../base/Kicker.vue'

import { useFixtureToc }   from '../../../lib/composables/fixture-toc'
import { useCurrentRule }  from '../../../lib/composables/route'

const currentRule = useCurrentRule()
const fixtureToc  = useFixtureToc()

const anchors = computed(() => {
  if (currentRule.value === null) return []
  return fixtureToc.value
    .filter(e => e.rule === currentRule.value!.slug)
    .map(e => ({ href: `#${e.id}`, title: e.title }))
})
</script>

<template>
  <div v-if="anchors.length" class="fixture-toc">
    <Kicker class="fixture-toc-kicker">Examples</Kicker>
    <ul class="fixture-toc-list">
      <li v-for="anchor in anchors" :key="anchor.href">
        <a :href="anchor.href">{{ anchor.title }}</a>
      </li>
    </ul>
  </div>
</template>
