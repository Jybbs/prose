<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'

import Disclosure    from '../base/Disclosure.vue'
import FixturePair   from './FixturePair.vue'
import FixtureToggle from './FixtureToggle.vue'

import { data as fixtures }   from '../../../data/fixtures.data'
import type { FixtureTab }    from '../../../lib/shared/fixture-tab'
import { registerFixture }    from '../../../lib/shared/fixture-toc'
import { lookup }             from '../../../lib/shared/lookup'

const props = defineProps<{
  case    : string
  open   ?: boolean
  rule    : string
  title  ?: string
  variant?: 'doc' | 'landing'
}>()

const rule       = lookup(fixtures, props.rule, 'Fixture rule')
const entry      = lookup(rule, props.case, `Fixture case under "${props.rule}"`)
const id         = computed(() => `fixture-${props.rule}-${props.case}`)
const activeTab  = ref<FixtureTab>('after')
const showToggle = computed(() => props.variant !== 'landing' && entry.changesSource)

let unregister: (() => void) | null = null
onMounted(() => {
  if (props.title) {
    unregister = registerFixture({ id: id.value, rule: props.rule, title: props.title })
  }
})
onUnmounted(() => unregister?.())
</script>

<template>
  <div v-if="!title" class="fixture">
    <header v-if="showToggle" class="fixture-bar">
      <FixtureToggle v-model="activeTab" />
    </header>
    <FixturePair
      :active-tab="activeTab"
      :input-html="entry.inputHtml"
      :output-html="entry.outputHtml"
      :variant="variant"
    />
  </div>

  <Disclosure v-else :id="id" :open="open">
    <template #title>{{ title }}</template>
    <template v-if="showToggle" #actions>
      <FixtureToggle v-model="activeTab" />
    </template>
    <FixturePair
      :active-tab="activeTab"
      :input-html="entry.inputHtml"
      :output-html="entry.outputHtml"
    />
  </Disclosure>
</template>
