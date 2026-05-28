<script setup lang="ts">
import { computed, ref } from 'vue'

import FixturePairDoc from '../fixtures/FixturePairDoc.vue'
import FixtureToggle  from '../fixtures/FixtureToggle.vue'

import { data as composition }  from '../../../data/composition.data'
import { data as fixturesData } from '../../../data/fixtures.data'
import { data as rules }        from '../../../data/rules.data'
import type { RenderedRule }    from '../../../data/rules.data'
import { railPaint }            from '../../../lib/shared/family-rail'
import type { FixtureTab }      from '../../../lib/shared/fixture-tab'
import { formatFolio }          from '../../../lib/shared/numerals'

interface RuleSegment {
  family : string | null
  index  : number
  rule   : RenderedRule | null
  slug   : string
}

interface CardRow {
  case           : string
  changesSource  : boolean
  dominantFamily : string | null
  inputHtml      : string
  num            : string
  outputHtml     : string
  railPaint      : string
  segments       : readonly RuleSegment[]
  title          : string
}

const cards = computed<readonly CardRow[]>(() =>
  composition.cases.map((entry, i) => {
    const families = entry.rules.map(slug => rules.bySlug[slug]?.family ?? null)
    const fixture  = fixturesData.composition?.[entry.case]
    return {
      case           : entry.case,
      changesSource  : fixture?.changesSource ?? false,
      dominantFamily : families[0] ?? null,
      inputHtml      : fixture?.inputHtml ?? '',
      num            : formatFolio(i + 1, 3),
      outputHtml     : fixture?.outputHtml ?? '',
      railPaint      : railPaint(families),
      segments       : entry.rules.map((slug, idx) => ({
        family : families[idx],
        index  : idx + 1,
        rule   : rules.bySlug[slug] ?? null,
        slug
      })),
      title          : entry.title
    }
  })
)

const activeCase = ref<string | null>(null)
const activeTab  = ref<FixtureTab>('after')

function toggle(row: CardRow): void {
  activeCase.value = activeCase.value === row.case ? null : row.case
}
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
      :style="{ '--rail-paint': row.railPaint }"
    >
      <div class="fixture-card-summary-row">
        <button
          type="button"
          class="fixture-card-summary"
          :aria-expanded="activeCase === row.case"
          :aria-controls="`composition-body-${row.case}`"
          @click="toggle(row)"
        >
          <span class="fixture-card-num">{{ row.num }}</span>
          <span class="fixture-card-title">{{ row.title }}</span>
        </button>
        <div
          class="fixture-card-actions"
          :class="{ 'is-active': activeCase === row.case }"
        >
          <FixtureToggle v-if="row.changesSource" v-model="activeTab" />
        </div>
      </div>
      <div
        :id="`composition-body-${row.case}`"
        class="fixture-card-body"
        role="region"
      >
        <div class="fixture-card-body-inner">
          <div v-if="activeCase === row.case" class="fixture-card-body-content">
            <ol class="composition-cards-rule-bar" aria-label="Rules in pipeline order">
              <li v-for="seg in row.segments" :key="seg.slug">
                <RuleTooltipPopper :rule="seg.rule">
                  <a
                    :href="`/rules/${seg.slug}`"
                    class="composition-cards-rule-chip"
                    :data-family="seg.family"
                    :title="seg.rule ? undefined : `${seg.slug} (${seg.family ?? 'undocumented'})`"
                  >
                    <span class="composition-cards-rule-num">{{ seg.index }}</span>
                    <span class="composition-cards-rule-slug">{{ seg.slug }}</span>
                  </a>
                </RuleTooltipPopper>
              </li>
            </ol>
            <FixturePairDoc
              :active-tab="activeTab"
              :input-html="row.inputHtml"
              :output-html="row.outputHtml"
            />
          </div>
        </div>
      </div>
    </li>
  </ol>
</template>
