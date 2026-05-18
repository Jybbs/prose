<script setup lang="ts">
import { computed } from 'vue'

import Kicker from '../base/Kicker.vue'

import { fixtureTocFor }   from '../../../lib/shared/fixture-toc'
import { useCurrentRule }  from '../../../lib/shared/route'

const currentRule = useCurrentRule()

const anchors = computed(() => {
  if (currentRule.value === null) return []
  return fixtureTocFor(currentRule.value.slug).map(e => ({
    href : `#${e.id}`,
    title: e.title
  }))
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
