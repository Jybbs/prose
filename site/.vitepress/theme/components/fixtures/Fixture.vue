<script setup lang="ts">
import { computed, defineAsyncComponent, onMounted, onUnmounted, ref } from 'vue'

import FixturePair   from './FixturePair.vue'
import FixtureToggle from './FixtureToggle.vue'

import { data as fixtures } from '../../../data/fixtures.data'
import type { FixtureTab }  from '../../../lib/shared/fixture-tab'
import { useFixtureToc }    from '../../../lib/composables/fixture-toc'
import { lookup }           from '../../../lib/shared/lookup'

const Disclosure = defineAsyncComponent(() => import('../base/Disclosure.vue'))

const props = defineProps<{
  case    : string
  open   ?: true
  rule    : string
  title  ?: string
  variant?: 'doc' | 'landing'
}>()

const rule       = lookup(fixtures, props.rule, 'Fixture rule')
const entry      = lookup(rule, props.case, `Fixture case under "${props.rule}"`)
const id         = computed(() => `fixture-${props.rule}-${props.case}`)
const activeTab  = ref<FixtureTab>('after')
const showToggle = computed(() => props.variant !== 'landing' && entry.changesSource)

const fixtureToc = useFixtureToc()
let unregister: (() => void) | null = null
onMounted(() => {
  if (props.title) {
    unregister = fixtureToc.register({ id: id.value, rule: props.rule, title: props.title })
  }
})
onUnmounted(() => unregister?.())
</script>

<template>
  <component
    :is="title ? Disclosure : 'div'"
    :class="title ? undefined : 'fixture'"
    :id="title ? id : undefined"
    :open="title ? open : undefined"
  >
    <template v-if="title" #title>{{ title }}</template>
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
