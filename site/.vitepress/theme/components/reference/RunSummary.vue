<script setup lang="ts">
interface SummaryLine {
  anchor : string
  text   : string
  tint   : 'apricot' | 'celadon'
}

const lines: readonly SummaryLine[] = [
  { anchor: '🪻', text: 'All clean.',                    tint: 'celadon' },
  { anchor: '☕', text: '5 diagnostics in 2 files.',     tint: 'apricot' },
  { anchor: '🗞️', text: 'Reformatted 4 files.',          tint: 'apricot' },
  { anchor: '🗞️', text: '3 files would be reformatted.', tint: 'apricot' }
]
</script>

<template>
  <div class="run-summary">
    <div class="run-summary-bar" aria-hidden="true">
      <span v-for="n in 3" :key="n" class="run-summary-dot" />
      <span class="run-summary-title">prose</span>
    </div>
    <div class="run-summary-body">
      <div v-for="line in lines" :key="line.text" class="run-summary-line">
        <span class="run-summary-anchor" aria-hidden="true">{{ line.anchor }}</span>
        <span class="run-summary-msg" :data-tint="line.tint">{{ line.text }}</span>
      </div>
    </div>
  </div>
</template>
