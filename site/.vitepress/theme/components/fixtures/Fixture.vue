<script setup lang="ts">
import { computed, defineAsyncComponent, ref } from 'vue'

import FixturePair   from './FixturePair.vue'
import FixtureToggle from './FixtureToggle.vue'

import { data as fixtures } from '../../../data/fixtures.data'
import type { FixtureTab }  from '../../../lib/shared/fixture-tab'
import { inlineCodeHtml }   from '../../../lib/shared/inline-code'
import { lookup }           from '../../../lib/shared/lookup'

const Disclosure = defineAsyncComponent(() => import('../base/Disclosure.vue'))

const props = defineProps<{
  case     : string
  open    ?: true
  rule     : string
  title   ?: string
  variant ?: 'doc' | 'landing'
}>()

const rule       = lookup(fixtures, props.rule, 'Fixture rule')
const entry      = lookup(rule, props.case, `Fixture case under "${props.rule}"`)
const id         = computed(() => `fixture-${props.rule}-${props.case}`)
const activeTab  = ref<FixtureTab>('after')
const showToggle = computed(() => props.variant !== 'landing' && entry.changesSource)
const titleHtml  = computed(() => props.title ? inlineCodeHtml(props.title) : '')
</script>

<template>
  <component
    :is="title ? Disclosure : 'div'"
    :class="title ? undefined : 'fixture'"
    :id="title ? id : undefined"
    :open="title ? open : undefined"
  >
    <template v-if="title" #title><span v-html="titleHtml" /></template>
    <template v-if="title && showToggle" #actions>
      <FixtureToggle v-model="activeTab" />
    </template>
    <header v-if="!title && showToggle" class="fixture-bar">
      <FixtureToggle v-model="activeTab" />
    </header>
    <FixturePair
      :active-tab="activeTab"
      :input-html="entry.inputHtml"
      :output-html="entry.outputHtml"
      :variant="title ? undefined : variant"
    />
  </component>
</template>
