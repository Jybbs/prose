<script setup lang="ts">
import { computed } from 'vue'

import { data, type GlanceSection } from '../../../lib/reference/section-glance.data'
import InlineProse                  from '../base/InlineProse.vue'

const props = defineProps<{ section: GlanceSection }>()

const entries = computed(() => data[props.section])
</script>

<template>
  <ul>
    <li v-for="entry in entries" :key="entry.href">
      <a :href="entry.href"><strong>{{ entry.title }}</strong></a> <InlineProse :nodes="entry.descriptionNodes" />
    </li>
  </ul>
</template>
