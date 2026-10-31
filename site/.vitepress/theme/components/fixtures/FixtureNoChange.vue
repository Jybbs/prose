<script setup lang="ts">
// Reserves the before/after toggle's exact footprint with an inert,
// hidden clone of its markup, then overlays the no-change chip so a
// no-edit card's actions slot lines up with a transforming card's.
</script>

<template>
  <div class="fixture-nochange relative inline-flex">
    <div class="invisible inline-flex rounded-sm border-1 border-divider bg-bg p-2" aria-hidden="true">
      <span class="fixture-toggle-pill toggle-pill" data-side="before">Before</span>
      <span class="fixture-toggle-pill toggle-pill" data-side="after">After</span>
    </div>
    <span class="pointer-events-none absolute inset-0 flex items-center justify-center change-chip">No Change</span>
  </div>
</template>
