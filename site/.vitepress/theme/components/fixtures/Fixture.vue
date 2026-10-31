<script setup lang="ts">
import { ref } from 'vue'

import FixturePairDoc from './FixturePairDoc.vue'
import FixtureToggle  from './FixtureToggle.vue'

import { useFixtureEntry } from '../../../lib/composables/use-fixture-entry'
import type { FixtureTab } from '../../../lib/shared/fixture-tab'
import InlineProse         from '../base/InlineProse.vue'

const props = defineProps<{
  case : string
  rule : string
}>()

const entry     = useFixtureEntry(props)
const activeTab = ref<FixtureTab>('after')
</script>

<template>
  <div class="fixture my-1-25rem">
    <div v-if="entry.descriptionNodes" class="fixture-lead mb-1rem">
      <InlineProse :nodes="entry.descriptionNodes" />
    </div>
    <header v-if="entry.hasToggle" class="fixture-bar flex justify-end pb-8">
      <FixtureToggle v-model="activeTab" />
    </header>
    <FixturePairDoc
      :active-tab="activeTab"
      :input-html="entry.inputHtml"
      :output-html="entry.outputHtml"
    />
  </div>
</template>
