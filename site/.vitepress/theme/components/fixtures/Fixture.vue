<script setup lang="ts">
import { useData }       from 'vitepress'
import { computed, ref } from 'vue'

import FixturePairDoc from './FixturePairDoc.vue'
import FixtureToggle  from './FixtureToggle.vue'

import { fixtureEntry }    from '../../../lib/fixtures/entry'
import type { FixtureTab } from '../../../lib/shared/fixture-tab'
import InlineProse         from '../base/InlineProse.vue'

const props = defineProps<{
  case : string
  rule : string
}>()

const { frontmatter } = useData()

const entry     = computed(() => fixtureEntry(frontmatter.value, props.rule, props.case))
const activeTab = ref<FixtureTab>('after')
</script>

<template>
  <div class="fixture">
    <div v-if="entry.descriptionNodes" class="fixture-lead">
      <InlineProse :nodes="entry.descriptionNodes" />
    </div>
    <header v-if="entry.hasToggle" class="fixture-bar">
      <FixtureToggle v-model="activeTab" />
    </header>
    <FixturePairDoc
      :active-tab="activeTab"
      :input-html="entry.inputHtml"
      :output-html="entry.outputHtml"
    />
  </div>
</template>
