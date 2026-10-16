<script setup lang="ts">
import NibBadge from '../base/NibBadge.vue'

import { platformEmblem } from '../../../lib/shared/emblems'
import { data }           from '../../../lib/usage/platforms.data'
</script>

<template>
  <table>
    <thead>
      <tr>
        <th>Triple</th>
        <th>Platform</th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="platform in data" :key="platform.target">
        <td><code>{{ platform.target }}</code></td>
        <td>
          <span class="nib-chip"><NibBadge :emblem="platformEmblem(platform.name)" :size="14" /></span>
          {{ platform.label }}<em v-if="platform.manylinux"> (glibc, manylinux)</em>
        </td>
      </tr>
    </tbody>
  </table>
</template>
