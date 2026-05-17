<script setup lang="ts">
import { data as codes } from '../../data/exit-codes.data'
</script>

<template>
  <div class="exit-codes">
    <details v-for="row in codes" :key="row.code" name="exit-code" class="exit-code-row">
      <summary class="exit-code-row-summary">
        <div class="exit-code-number">{{ row.code }}</div>
        <div class="exit-code-body">
          <div class="exit-code-label">{{ row.label }}</div>
          <p class="exit-code-summary">{{ row.summary }}</p>
        </div>
      </summary>
      <div class="exit-code-detail">
        <ul>
          <li v-for="(html, idx) in row.detailHtml" :key="idx" v-html="html" />
        </ul>
      </div>
    </details>
  </div>
</template>
