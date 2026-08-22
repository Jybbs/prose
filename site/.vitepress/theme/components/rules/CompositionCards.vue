<script setup lang="ts">
import { computed, ref } from 'vue'

import FixtureNoChange from '../fixtures/FixtureNoChange.vue'
import FixturePairDoc  from '../fixtures/FixturePairDoc.vue'
import FixtureToggle   from '../fixtures/FixtureToggle.vue'
import RuleSegmentChip from './RuleSegmentChip.vue'

import { useHashOpen }                    from '../../../lib/composables/use-hash-open'
import { fixtureEntry }                   from '../../../lib/fixtures/entry'
import type { FixtureEntry }              from '../../../lib/fixtures/fixtures.data'
import type { CompositionCase }           from '../../../lib/rules/composition'
import { data as composition }            from '../../../lib/rules/composition.data'
import { casesForRule, type RuleSegment } from '../../../lib/rules/rule-view'
import { data as rules }                  from '../../../lib/rules/rules.data'
import type { SavedSession }              from '../../../lib/sandbox/session'
import { railPaint }                      from '../../../lib/shared/family-rail'
import type { FixtureTab }                from '../../../lib/shared/fixture-tab'
import { inlineCode }                     from '../../../lib/shared/inline-code'
import InlineProse                        from '../base/InlineProse.vue'

interface CardRow extends FixtureEntry {
  case           : string
  dominantFamily : string | null
  headlinePaint  : string
  railPaint      : string
  sandboxSeed    : SavedSession
  segments       : readonly RuleSegment[]
  titleHtml      : string
}

const props = defineProps<{ rule?: string }>()

function toCardRow(entry: CompositionCase): CardRow {
  const families = entry.rules.map(slug => rules.bySlug[slug]?.family ?? null)
  return {
    ...fixtureEntry('composition', entry.case),
    case           : entry.case,
    dominantFamily : families[0] ?? null,
    headlinePaint  : railPaint(families, 'to right'),
    railPaint      : railPaint(families),
    sandboxSeed    : { configToml: entry.configToml, source: entry.source },
    titleHtml      : inlineCode(entry.title),
    segments       : entry.rules.map((slug, idx) => ({
      family : families[idx] ?? null,
      index  : idx + 1,
      rule   : rules.bySlug[slug] ?? null,
      slug
    }))
  }
}

// A rule page renders the subset the rule participates in. Each card leaves
// its folio empty so the `prose-fixture-folio` counter numbers it in place,
// continuing the run the fixtures above it started.
const cards = computed<readonly CardRow[]>(() => {
  const participating = props.rule === undefined ? null : new Set(casesForRule(props.rule))
  return composition.cases
    .filter(entry => participating === null || participating.has(entry.case))
    .map(toCardRow)
})

const activeCase = ref<string | null>(null)
const activeTab  = ref<FixtureTab>('after')

function toggle(row: CardRow): void {
  activeCase.value = activeCase.value === row.case ? null : row.case
}

useHashOpen(fragment => {
  if (cards.value.some(row => row.case === fragment)) activeCase.value = fragment
})
</script>

<template>
  <ol class="composition-cards-list" aria-label="Composition cards">
    <li
      v-for="row in cards"
      :key="row.case"
      :id="row.case"
      class="fixture-card"
      :class="{ 'is-open': activeCase === row.case }"
      :data-family="row.dominantFamily"
      :data-edits="row.changesSource"
      :style="{ '--rail-paint': row.railPaint, '--headline-paint': row.headlinePaint }"
      @click="toggle(row)"
    >
      <div class="fixture-card-summary-row">
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
            v-show="activeCase !== row.case"
            class="composition-cards-ticks"
            :aria-label="`${row.segments.length} rules in pipeline order`"
          >
            <li
              v-for="seg in row.segments"
              :key="seg.slug"
              class="composition-cards-tick-item"
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
            <ol
              class="composition-cards-bar"
              :class="{ 'is-open': activeCase === row.case }"
              :aria-label="`${row.segments.length} rules in pipeline order`"
            >
              <li
                v-for="seg in row.segments"
                :key="seg.slug"
                class="composition-cards-bar-cell"
                @click.stop
              >
                <RuleSegmentChip :segment="seg" :with-tooltip="true" />
              </li>
            </ol>
          </div>
        </div>
      </div>
    </li>
  </ol>
</template>
