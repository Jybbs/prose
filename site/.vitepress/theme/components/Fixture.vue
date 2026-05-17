<script setup lang="ts">
import Disclosure  from './Disclosure.vue'
import FixturePair from './FixturePair.vue'

import { lookup }           from '../../lib/registry'
import { data as fixtures } from '../../data/fixtures.data'

const props = defineProps<{
  case    : string
  open   ?: boolean
  rule    : string
  title  ?: string
  variant?: 'doc' | 'landing'
}>()

const rule  = lookup(fixtures, props.rule, 'Fixture rule')
const entry = lookup(rule, props.case, `Fixture case under "${props.rule}"`)
</script>

<template>
  <FixturePair
    v-if="!title"
    :variant="variant"
    :input-html="entry.inputHtml"
    :output-html="entry.outputHtml"
  />
  <Disclosure v-else variant="fixture" :open="open">
    <template #title>{{ title }}</template>
    <FixturePair
      :input-html="entry.inputHtml"
      :output-html="entry.outputHtml"
    />
  </Disclosure>
</template>
