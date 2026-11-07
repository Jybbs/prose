<script setup lang="ts">
import { useResizeObserver }                       from '@vueuse/core'
import { useData }                                 from 'vitepress'
import { computed, nextTick, ref, useTemplateRef } from 'vue'

import { Disclosure }  from '../../../kit'
import FixtureNoChange from '../fixtures/FixtureNoChange.vue'
import FixturePairDoc  from '../fixtures/FixturePairDoc.vue'
import FixtureToggle   from '../fixtures/FixtureToggle.vue'
import RuleSegmentChip from './RuleSegmentChip.vue'

import { useHashOpen }                            from '../../../lib/composables/use-hash-open'
import { COMPOSITION_RULE, fixtureEntry }         from '../../../lib/fixtures/entry'
import type { FixtureEntry }                      from '../../../lib/fixtures/entry'
import type { CompositionCase }                   from '../../../lib/rules/composition'
import { deckTravel }                             from '../../../lib/rules/deck-travel'
import { pageCompositionCases, type RuleSegment } from '../../../lib/rules/rule-view'
import { data as rules }                          from '../../../lib/rules/rules.data'
import type { SavedSession }                      from '../../../lib/sandbox/session'
import type { FixtureTab }                        from '../../../lib/shared/fixture-tab'
import { InlineProse }                            from '../base/InlineProse'

interface CardRow extends FixtureEntry {
  case        : string
  sandboxSeed : SavedSession
  segments    : readonly RuleSegment[]
  titleHtml   : string
}

const props = defineProps<{ rule?: string }>()

const { frontmatter } = useData()

function toCardRow(entry: CompositionCase): CardRow {
  return {
    ...fixtureEntry(frontmatter.value, COMPOSITION_RULE, entry.case),
    case        : entry.case,
    sandboxSeed : { configToml: entry.configToml, source: entry.source },
    titleHtml   : entry.titleHtml,
    segments    : entry.rules.map((slug, idx) => {
      const rule = rules.bySlug[slug] ?? null
      return { family: rule?.family ?? null, index: idx + 1, rule, slug }
    })
  }
}

// A rule page renders the subset the rule participates in, and every page
// leaves out the cases it embeds inline. Each card leaves its folio empty so
// the `prose-fixture-folio` counter numbers it in place, continuing the run the
// fixtures above it started.
const cards = computed<readonly CardRow[]>(() =>
  pageCompositionCases(frontmatter.value, props.rule).map(toCardRow))

const activeCase = ref<string | null>(null)
const activeTab  = ref<FixtureTab>('after')

const list = useTemplateRef<HTMLElement>('list')
const bars = useTemplateRef<HTMLElement[]>('bars')

function toggle(row: CardRow): void {
  void deckTravel(list.value!, async () => {
    activeCase.value = activeCase.value === row.case ? null : row.case
    await nextTick()
  })
}

// Holds each card's foot open for its deck, writing the height the deck takes
// once its boxes spread and name their rules.
useResizeObserver(bars, entries => {
  for (const { target } of entries) {
    target.closest<HTMLElement>('li.fixture-card')?.style.setProperty('--deck-bar-h', `${(target as HTMLElement).offsetHeight}px`)
  }
})

useHashOpen(fragment => {
  if (cards.value.some(row => row.case === fragment)) activeCase.value = fragment
})
</script>

<template>
  <ol ref="list" class="composition-cards-list m-0 list-none p-0" aria-label="Composition cards">
    <Disclosure
      v-for="row in cards"
      :key="row.case"
      v-slot="disclosure"
      :open="activeCase === row.case"
      @update:open="toggle(row)"
    >
      <li
        :id="row.case"
        class="fixture-card group/fixture cursor-pointer"
        :data-open="activeCase === row.case || undefined"
        :data-edits="row.changesSource"
        @click="toggle(row)"
      >
        <div class="fixture-card-summary-row max-mobile:group-data-open/fixture:gap-y-0">
          <span class="composition-cards-slot col-1 row-1 mt-7 ml-8 h-20 self-start" aria-hidden="true" :style="{ '--deck-count': row.segments.length }"></span>
          <button
            v-bind="disclosure.trigger"
            type="button"
            class="fixture-card-summary col-2 row-1 max-mobile:group-data-open/fixture:col-start-2 max-mobile:group-data-open/fixture:row-1 max-mobile:group-data-open/fixture:-col-end-1"
            @click.stop
          >
            <span class="fixture-card-num" aria-hidden="true" />
            <span class="fixture-card-title text-balance" v-html="row.titleHtml" />
          </button>
          <div class="contents">
            <ol
              class="composition-cards-ticks m-0 list-none p-0"
              :aria-label="`${row.segments.length} rules in pipeline order`"
              :style="{ '--deck-count': row.segments.length }"
            >
              <li
                v-for="(seg, at) in row.segments"
                :key="seg.slug"
                class="composition-cards-tick-item my-0 list-none ps-0"
                :style="{ '--deck-at': at }"
                @click.stop
              >
                <RuleSegmentChip :segment="seg" :with-tooltip="false" />
              </li>
            </ol>
            <div v-show="activeCase === row.case" class="col-3 row-1 inline-flex items-center max-mobile:group-data-open/fixture:col-start-2 max-mobile:group-data-open/fixture:row-2 max-mobile:group-data-open/fixture:-col-end-1 max-mobile:group-data-open/fixture:min-h-0 max-mobile:group-data-open/fixture:justify-start max-mobile:group-data-open/fixture:justify-self-start max-mobile:group-data-open/fixture:p-0 max-mobile:group-data-open/fixture:pb-8" @click.stop>
              <FixtureToggle v-if="row.hasToggle" v-model="activeTab" />
              <FixtureNoChange v-else />
            </div>
          </div>
        </div>

        <div
          v-bind="disclosure.content"
          class="fixture-card-body transition-grid-template-rows duration-grow ease-morph"
          :hidden="undefined"
          @click.stop
        >
          <div class="fixture-card-body-inner">
            <div class="fixture-card-body-content">
              <template v-if="row.descriptionNodes">
                <div class="fixture-card-desc"><InlineProse :nodes="row.descriptionNodes" /></div>
                <div class="fixture-card-rule" aria-hidden="true" />
              </template>
              <FixturePairDoc
                v-if="activeCase === row.case"
                :active-tab="activeTab"
                :input-html="row.inputHtml"
                :output-html="row.outputHtml"
                :sandbox-seed="row.sandboxSeed"
              />
              <ol ref="bars" class="composition-cards-bar invisible mt-12 mb-0 flex list-none flex-wrap gap-6 p-0" aria-hidden="true" inert>
                <li v-for="seg in row.segments" :key="seg.slug" class="my-0 flex min-w-0 flex-none list-none ps-0">
                  <RuleSegmentChip :segment="seg" :with-tooltip="false" />
                </li>
              </ol>
            </div>
          </div>
        </div>
      </li>
    </Disclosure>
  </ol>
</template>
