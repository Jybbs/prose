<script setup lang="ts">
import { computed } from 'vue'

import { data as rows } from '../../../lib/primitives/primitive-surface.data'

import type { PrimitiveStability } from '../../../lib/shared/registries'
import InlineProse                 from '../base/InlineProse.vue'

const props = defineProps<{ stability: PrimitiveStability }>()

const listed = computed(() =>
  rows.filter(row => row.stability === props.stability)
      .toSorted((a, b) => a.slug.localeCompare(b.slug))
)
</script>

<template>
  <table>
    <thead>
      <tr>
        <th>Primitive</th>
        <th>Role</th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="row in listed" :key="row.slug">
        <td><InlineProse :nodes="row.linkNodes" /></td>
        <td><InlineProse :nodes="row.summaryNodes" /></td>
      </tr>
    </tbody>
  </table>
</template>
