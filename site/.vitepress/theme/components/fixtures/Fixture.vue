<script setup lang="ts">
import Disclosure  from '../base/Disclosure.vue'
import FixturePair from './FixturePair.vue'

import { data as fixtures } from '../../../data/fixtures.data'
import { lookup }           from '../../../lib/shared/registry'

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
    :input-html="entry.inputHtml"
    :output-html="entry.outputHtml"
    :variant="variant"
  />
  <Disclosure v-else :open="open" variant="fixture">
    <template #title>{{ title }}</template>
    <FixturePair
      :input-html="entry.inputHtml"
      :output-html="entry.outputHtml"
    />
  </Disclosure>
</template>
