<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue'

import Disclosure  from '../base/Disclosure.vue'
import FixturePair from './FixturePair.vue'

import { data as fixtures }   from '../../../data/fixtures.data'
import { registerFixture }    from '../../../lib/shared/fixture-toc'
import { lookup }             from '../../../lib/shared/lookup'

const props = defineProps<{
  case    : string
  open   ?: boolean
  rule    : string
  title  ?: string
  variant?: 'doc' | 'landing'
}>()

const rule  = lookup(fixtures, props.rule, 'Fixture rule')
const entry = lookup(rule, props.case, `Fixture case under "${props.rule}"`)
const id    = computed(() => `fixture-${props.rule}-${props.case}`)

let unregister: (() => void) | null = null
onMounted(() => {
  if (props.title) {
    unregister = registerFixture({ id: id.value, rule: props.rule, title: props.title })
  }
})
onUnmounted(() => unregister?.())
</script>

<template>
  <FixturePair
    v-if="!title"
    :input-html="entry.inputHtml"
    :output-html="entry.outputHtml"
    :variant="variant"
  />
  <Disclosure v-else :id="id" :open="open" variant="fixture">
    <template #title>{{ title }}</template>
    <FixturePair
      :input-html="entry.inputHtml"
      :output-html="entry.outputHtml"
    />
  </Disclosure>
</template>
