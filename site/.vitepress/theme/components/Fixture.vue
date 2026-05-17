<script setup lang="ts">
import FixturePair from './FixturePair.vue'

import { data as fixtures } from '../../data/fixtures.data'

const props = defineProps<{
  case    : string
  open   ?: boolean
  rule    : string
  title  ?: string
  variant?: 'doc' | 'landing'
}>()

const entry = fixtures[props.rule]?.[props.case]
if (!entry) {
  const cases = Object.keys(fixtures[props.rule] ?? {}).sort().join(', ')
  throw new Error(
    `Fixture "${props.rule}/${props.case}" not found under tests/fixtures/. ` +
    `Available cases for "${props.rule}": ${cases || '(rule not registered)'}`
  )
}
</script>

<template>
  <div v-if="variant === 'landing'" class="fixture fixture-landing">
    <div class="fixture-landing-side">
      <div class="fixture-landing-label">
        <span class="fixture-landing-dot fixture-landing-dot-before" />
        <span>Before</span>
      </div>
      <div class="fixture-landing-code" v-html="entry.inputHtml" />
    </div>
    <div class="fixture-landing-arrow" aria-hidden="true">→</div>
    <div class="fixture-landing-side">
      <div class="fixture-landing-label">
        <span class="fixture-landing-dot fixture-landing-dot-after" />
        <span>After</span>
      </div>
      <div class="fixture-landing-code" v-html="entry.outputHtml" />
    </div>
  </div>
  <div v-else-if="!title" class="fixture">
    <FixturePair :input-html="entry.inputHtml" :output-html="entry.outputHtml" />
  </div>
  <details v-else class="fixture-disclosure" :open="open">
    <summary class="fixture-disclosure-summary">
      <span class="fixture-disclosure-caret" aria-hidden="true">›</span>
      <span class="fixture-disclosure-title">{{ title }}</span>
    </summary>
    <div class="fixture fixture-disclosure-body">
      <FixturePair :input-html="entry.inputHtml" :output-html="entry.outputHtml" />
    </div>
  </details>
</template>
