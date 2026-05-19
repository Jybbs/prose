<script setup lang="ts">
import type { FixtureTab } from '../../../lib/shared/fixture-tab'

const tab = defineModel<FixtureTab>({ required: true })
</script>

<template>
  <div class="fixture-toggle" role="tablist" aria-label="Fixture view">
    <button role="tab" class="fixture-toggle-pill" data-side="before" :aria-selected="tab === 'before'" @click.stop="tab = 'before'">Before</button>
    <button role="tab" class="fixture-toggle-pill" data-side="after"  :aria-selected="tab === 'after'"  @click.stop="tab = 'after'">After</button>
  </div>
</template>
