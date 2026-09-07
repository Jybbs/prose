<script setup lang="ts">
import { useToggle }     from '@vueuse/core'
import { useData }       from 'vitepress'
import { computed, ref } from 'vue'

import FixtureNoChange from './FixtureNoChange.vue'
import FixturePairDoc  from './FixturePairDoc.vue'
import FixtureToggle   from './FixtureToggle.vue'

import { useHashOpen }     from '../../../lib/composables/use-hash-open'
import { fixtureEntry }    from '../../../lib/fixtures/entry'
import { data as rules }   from '../../../lib/rules/rules.data'
import type { FixtureTab } from '../../../lib/shared/fixture-tab'
import { ruleSlug }        from '../../../lib/shared/rule-slug'
import InlineProse         from '../base/InlineProse.vue'

const props = defineProps<{
  case      : string
  rule      : string
  titleHtml : string
}>()

const { frontmatter } = useData()

const entry     = computed(() => fixtureEntry(frontmatter.value, props.rule, props.case))
const id        = computed(() => `fixture-${props.rule}-${props.case}`)
const activeTab = ref<FixtureTab>('after')

const family = computed(() => rules.bySlug[ruleSlug(props.rule)]?.family ?? null)

const [isOpen, toggle] = useToggle(false)

useHashOpen(fragment => { if (fragment === id.value) isOpen.value = true })
</script>

<template>
  <section
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
</template>
