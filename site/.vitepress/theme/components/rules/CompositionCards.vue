<script setup lang="ts">
import { useResizeObserver }                      from '@vueuse/core'
import { useData }                                from 'vitepress'
import { computed, nextTick, ref, useTemplateRef } from 'vue'

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
import InlineProse                                from '../base/InlineProse.vue'

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
  <ol ref="list" class="composition-cards-list" aria-label="Composition cards">
    <li
      v-for="row in cards"
      :key="row.case"
      :id="row.case"
      class="fixture-card"
      :class="{ 'is-open': activeCase === row.case }"
      :data-edits="row.changesSource"
      @click="toggle(row)"
    >
      <div class="fixture-card-summary-row">
        <span class="composition-cards-slot" aria-hidden="true" :style="{ '--deck-count': row.segments.length }"></span>
        <button
          type="button"
          class="fixture-card-summary"
          :aria-expanded="activeCase === row.case"
          :aria-controls="`composition-body-${row.case}`"
        >
          <span class="fixture-card-num" aria-hidden="true" />
          <span class="fixture-card-title" v-html="row.titleHtml" />
        </button>
        <div class="composition-cards-tick-cell">
          <ol
            class="composition-cards-ticks"
            :aria-label="`${row.segments.length} rules in pipeline order`"
            :style="{ '--deck-count': row.segments.length }"
          >
            <li
              v-for="(seg, at) in row.segments"
              :key="seg.slug"
              class="composition-cards-tick-item"
              :style="{ '--deck-at': at }"
              @click.stop
            >
              <RuleSegmentChip :segment="seg" :with-tooltip="false" />
            </li>
          </ol>
          <div v-show="activeCase === row.case" class="composition-cards-toggle-slot" @click.stop>
            <FixtureToggle v-if="row.hasToggle" v-model="activeTab" />
            <FixtureNoChange v-else />
          </div>
        </div>
      </div>

      <div
        :id="`composition-body-${row.case}`"
        class="fixture-card-body"
        role="region"
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
            <ol ref="bars" class="composition-cards-bar" aria-hidden="true" inert>
              <li v-for="seg in row.segments" :key="seg.slug" class="composition-cards-bar-cell">
                <RuleSegmentChip :segment="seg" :with-tooltip="false" />
              </li>
            </ol>
          </div>
        </div>
      </div>
    </li>
  </ol>
</template>
