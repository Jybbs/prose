<script setup lang="ts">
import { Tabs } from '../../../kit'

import type { FixtureTab } from '../../../lib/shared/fixture-tab'

const tab = defineModel<FixtureTab>({ required: true })

// Unbinds `aria-controls` from each trigger because the pane it would name
// renders in `FixturePairDoc`, outside this list.
</script>

<template>
  <Tabs v-model="tab" v-slot="tabs" label="Fixture view">
    <div v-bind="tabs.list" class="inline-flex rounded-sm border-1 border-divider bg-bg p-2">
      <button v-bind="tabs.trigger('before')" type="button" class="fixture-toggle-pill toggle-pill" data-side="before" :aria-controls="undefined" @click.stop>Before</button>
      <button v-bind="tabs.trigger('after')" type="button" class="fixture-toggle-pill toggle-pill" data-side="after" :aria-controls="undefined" @click.stop>After</button>
    </div>
  </Tabs>
</template>
