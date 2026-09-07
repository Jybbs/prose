<script setup lang="ts">
import { data } from '../../../lib/usage/platforms.data'
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
        <td>{{ platform.label }}<em v-if="platform.manylinux"> (glibc, manylinux)</em></td>
      </tr>
    </tbody>
  </table>
</template>
