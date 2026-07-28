<script setup lang="ts">
import { useEventListener, useToggle } from '@vueuse/core'
import { computed, onMounted, ref }    from 'vue'

import FixtureNoChange    from './FixtureNoChange.vue'
import FixturePairDoc     from './FixturePairDoc.vue'
import FixturePairLanding from './FixturePairLanding.vue'
import FixtureToggle      from './FixtureToggle.vue'

import { fixtureEntry }    from '../../../lib/fixtures/entry'
import { data as rules }   from '../../../lib/rules/rules.data'
import type { FixtureTab } from '../../../lib/shared/fixture-tab'
import { inlineCode }      from '../../../lib/shared/inline-code'
import InlineProse         from '../base/InlineProse.vue'

const props = defineProps<{
  case     : string
  open    ?: true
  rule     : string
  title   ?: string
  variant ?: 'doc' | 'landing'
}>()

const entry      = fixtureEntry(props.rule, props.case)
const id         = computed(() => `fixture-${props.rule}-${props.case}`)
const activeTab  = ref<FixtureTab>('after')
const showToggle = computed(() => props.variant !== 'landing' && entry.hasToggle)
const titleHtml  = computed(() => props.title ? inlineCode(props.title) : '')

const ruleData = computed(() => rules.bySlug[props.rule.replaceAll('_', '-')] ?? null)
const family   = computed(() => ruleData.value?.family ?? null)

const [isOpen, toggle] = useToggle(props.open === true)

function syncWithHash(): void {
  if (window.location.hash === `#${id.value}`) {
    isOpen.value = true
  }
}

onMounted(syncWithHash)
useEventListener('hashchange', syncWithHash)
</script>

<template>
  <section
    v-if="title"
    :id="id"
    class="fixture-card"
    :class="{ 'is-open': isOpen }"
    :data-family="family"
    :data-edits="entry.changesSource"
    :data-lint="entry.hasFindings"
  >
    <div class="fixture-card-summary-row" @click="toggle()">
      <button
        type="button"
        class="fixture-card-summary"
        :aria-expanded="isOpen"
        :aria-controls="`${id}-body`"
      >
        <span class="fixture-card-num" aria-hidden="true" />
        <span class="fixture-card-title" v-html="titleHtml" />
      </button>
      <div
        class="fixture-card-actions"
        :class="{ 'is-active': isOpen }"
        @click.stop
      >
        <FixtureToggle v-if="entry.hasToggle" v-model="activeTab" />
        <FixtureNoChange v-else />
      </div>
    </div>
    <div
      :id="`${id}-body`"
      class="fixture-card-body"
      role="region"
    >
      <div class="fixture-card-body-inner">
        <div class="fixture-card-body-content">
          <template v-if="entry.descriptionNodes">
            <div class="fixture-card-desc"><InlineProse :nodes="entry.descriptionNodes" /></div>
            <div class="fixture-card-rule" aria-hidden="true" />
          </template>
          <FixturePairDoc
            v-if="isOpen"
            :active-tab="activeTab"
            :input-html="entry.inputHtml"
            :output-html="entry.outputHtml"
          />
        </div>
      </div>
    </div>
  </section>

  <div v-else class="fixture">
    <div v-if="entry.descriptionNodes && variant !== 'landing'" class="fixture-lead">
      <InlineProse :nodes="entry.descriptionNodes" />
    </div>
    <header v-if="showToggle" class="fixture-bar">
      <FixtureToggle v-model="activeTab" />
    </header>
    <FixturePairLanding
      v-if="variant === 'landing'"
      :input-html="entry.inputHtml"
      :output-html="entry.outputHtml"
    />
    <FixturePairDoc
      v-else
      :active-tab="activeTab"
      :input-html="entry.inputHtml"
      :output-html="entry.outputHtml"
    />
  </div>
</template>
