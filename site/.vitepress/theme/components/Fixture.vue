<script setup lang="ts">
import { data as fixtures } from '../../data/fixtures.data'

const props = defineProps<{
  case  : string
  open ?: boolean
  rule  : string
  title?: string
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
  <div v-if="!title" class="fixture">
    <div class="fixture-side">
      <div class="fixture-label">Before</div>
      <div class="fixture-code" v-html="entry.inputHtml" />
    </div>
    <div class="fixture-arrow" aria-hidden="true">→</div>
    <div class="fixture-side">
      <div class="fixture-label">After</div>
      <div class="fixture-code" v-html="entry.outputHtml" />
    </div>
  </div>
  <details v-else class="fixture-disclosure" :open="open">
    <summary class="fixture-disclosure-summary">
      <span class="fixture-disclosure-caret" aria-hidden="true">›</span>
      <span class="fixture-disclosure-title">{{ title }}</span>
    </summary>
    <div class="fixture fixture-disclosure-body">
      <div class="fixture-side">
        <div class="fixture-label">Before</div>
        <div class="fixture-code" v-html="entry.inputHtml" />
      </div>
      <div class="fixture-arrow" aria-hidden="true">→</div>
      <div class="fixture-side">
        <div class="fixture-label">After</div>
        <div class="fixture-code" v-html="entry.outputHtml" />
      </div>
    </div>
  </details>
</template>
