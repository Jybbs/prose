<script setup lang="ts">
// Reserves the before/after toggle's exact footprint with an inert,
// hidden clone of its markup, then overlays the gray no-change chip, so
// a no-edit card's actions slot lines up with a transforming card's.
</script>

<template>
  <div class="fixture-nochange">
    <div class="fixture-toggle fixture-nochange-ghost" aria-hidden="true">
      <span class="fixture-toggle-pill" data-side="before">Before</span>
      <span class="fixture-toggle-pill" data-side="after">After</span>
    </div>
    <span class="fixture-nochange-label">No Change</span>
  </div>
</template>
