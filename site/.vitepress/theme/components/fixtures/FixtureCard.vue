<script setup lang="ts">
import { useToggle }     from '@vueuse/core'
import { computed, ref } from 'vue'

import { Disclosure, NibBadge } from '../../../kit'
import FixtureNoChange          from './FixtureNoChange.vue'
import FixturePairDoc           from './FixturePairDoc.vue'
import FixtureToggle            from './FixtureToggle.vue'

import { useFixtureEntry } from '../../../lib/composables/use-fixture-entry'
import { useHashOpen }     from '../../../lib/composables/use-hash-open'
import { data as rules }   from '../../../lib/rules/rules.data'
import { familyEmblem }    from '../../../lib/shared/emblems'
import type { FixtureTab } from '../../../lib/shared/fixture-tab'
import { ruleSlug }        from '../../../lib/shared/rule-slug'
import { InlineProse }     from '../base/InlineProse'

const props = defineProps<{
  case      : string
  rule      : string
  titleHtml : string
}>()

const entry     = useFixtureEntry(props)
const id        = computed(() => `fixture-${props.rule}-${props.case}`)
const activeTab = ref<FixtureTab>('after')

const family = computed(() => rules.bySlug[ruleSlug(props.rule)]?.family ?? null)

const [isOpen, toggle] = useToggle(false)

useHashOpen(fragment => { if (fragment === id.value) isOpen.value = true })
</script>

<template>
  <Disclosure v-model:open="isOpen" v-slot="disclosure">
    <section
      :id="id"
      class="fixture-card group/fixture"
      :data-open="isOpen || undefined"
      :data-family="family"
      :data-edits="entry.changesSource"
      :data-lint="entry.hasFindings"
    >
      <div class="fixture-card-summary-row max-mobile:group-data-open/fixture:gap-y-0" @click="toggle()">
        <span class="mt-7 mr-12 ml-8 flex size-20 items-center justify-center self-start max-mobile:mr-0" aria-hidden="true">
          <NibBadge v-if="family" class="block text-family-nib-card group-data-[edits=false]/fixture:not-group-data-[lint=true]/fixture:text-text-3-card-31" :emblem="familyEmblem(family)" :size="14" />
        </span>
        <button
          v-bind="disclosure.trigger"
          type="button"
          class="fixture-card-summary max-mobile:group-data-open/fixture:col-start-2 max-mobile:group-data-open/fixture:row-1 max-mobile:group-data-open/fixture:-col-end-1"
          @click.stop
        >
          <span class="fixture-card-num" aria-hidden="true" />
          <span class="fixture-card-title" v-html="titleHtml" />
        </button>
        <div
          class="hidden items-center group-data-open/fixture:inline-flex group-data-[edits=false]/fixture:not-group-data-[lint=true]/fixture:inline-flex max-mobile:group-data-open/fixture:col-start-2 max-mobile:group-data-open/fixture:row-2 max-mobile:group-data-open/fixture:-col-end-1 max-mobile:group-data-open/fixture:min-h-0 max-mobile:group-data-open/fixture:justify-start max-mobile:group-data-open/fixture:justify-self-start max-mobile:group-data-open/fixture:p-0 max-mobile:group-data-open/fixture:pb-8"
          @click.stop
        >
          <FixtureToggle v-if="entry.hasToggle" v-model="activeTab" />
          <FixtureNoChange v-else />
        </div>
      </div>
      <div
        v-bind="disclosure.content"
        class="fixture-card-body transition-grid-template-rows"
        :hidden="undefined"
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
  </Disclosure>
</template>
