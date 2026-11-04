<script setup lang="ts">
import type { FixtureTab } from '../../../lib/shared/fixture-tab'

const tab = defineModel<FixtureTab>({ required: true })
</script>

<template>
  <div class="inline-flex rounded-sm border-1 border-divider bg-bg p-2" role="tablist" aria-label="Fixture view">
    <button type="button" role="tab" class="fixture-toggle-pill toggle-pill" data-side="before" :aria-selected="tab === 'before'" @click.stop="tab = 'before'">Before</button>
    <button type="button" role="tab" class="fixture-toggle-pill toggle-pill" data-side="after"  :aria-selected="tab === 'after'"  @click.stop="tab = 'after'">After</button>
  </div>
</template>
