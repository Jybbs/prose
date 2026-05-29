<script setup lang="ts">
import { useEventListener }         from '@vueuse/core'
import { computed, onMounted, ref } from 'vue'

import FixtureNoChange    from './FixtureNoChange.vue'
import FixturePairDoc     from './FixturePairDoc.vue'
import FixturePairLanding from './FixturePairLanding.vue'
import FixtureToggle      from './FixtureToggle.vue'

import { data as fixtures } from '../../../data/fixtures.data'
import { data as rules }    from '../../../data/rules.data'
import type { FixtureTab }  from '../../../lib/shared/fixture-tab'
import { lookup }           from '../../../lib/shared/lookup'

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
const titleHtml  = computed(() => props.title ? props.title.replace(/`([^`]+)`/g, '<code>$1</code>') : '')

const ruleData = computed(() => rules.bySlug[props.rule.replaceAll('_', '-')] ?? null)
const family   = computed(() => ruleData.value?.family ?? null)

const isOpen = ref<boolean>(props.open === true)

function toggle(): void {
  isOpen.value = !isOpen.value
}

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
  >
    <div class="fixture-card-summary-row">
      <button
        type="button"
        class="fixture-card-summary"
        :aria-expanded="isOpen"
        :aria-controls="`${id}-body`"
        @click="toggle"
      >
        <span class="fixture-card-num" aria-hidden="true" />
        <span class="fixture-card-title" v-html="titleHtml" />
      </button>
      <div
        class="fixture-card-actions"
        :class="{ 'is-active': isOpen }"
      >
        <FixtureToggle v-if="entry.changesSource" v-model="activeTab" />
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
          <template v-if="entry.descriptionHtml">
            <div class="fixture-card-desc" v-html="entry.descriptionHtml" />
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
    <div
      v-if="entry.descriptionHtml && variant !== 'landing'"
      class="fixture-lead"
      v-html="entry.descriptionHtml"
    />
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
