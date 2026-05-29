<script setup lang="ts">
import { computed, ref } from 'vue'

import FixtureNoChange from '../fixtures/FixtureNoChange.vue'
import FixturePairDoc  from '../fixtures/FixturePairDoc.vue'
import FixtureToggle   from '../fixtures/FixtureToggle.vue'

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
  case            : string
  changesSource   : boolean
  descriptionHtml : string | undefined
  dominantFamily  : string | null
  headlinePaint   : string
  inputHtml       : string
  num             : string
  outputHtml      : string
  railPaint       : string
  segments        : readonly RuleSegment[]
  title           : string
}

const cards = computed<readonly CardRow[]>(() =>
  composition.cases.map((entry, i) => {
    const families = entry.rules.map(slug => rules.bySlug[slug]?.family ?? null)
    const fixture  = fixturesData.composition?.[entry.case]
    return {
      case            : entry.case,
      changesSource   : fixture?.changesSource ?? false,
      descriptionHtml : fixture?.descriptionHtml,
      dominantFamily  : families[0] ?? null,
      headlinePaint   : railPaint(families, 'to right'),
      inputHtml       : fixture?.inputHtml ?? '',
      num             : formatFolio(i + 1, 3),
      outputHtml      : fixture?.outputHtml ?? '',
      railPaint       : railPaint(families),
      segments        : entry.rules.map((slug, idx) => ({
        family : families[idx] ?? null,
        index  : idx + 1,
        rule   : rules.bySlug[slug] ?? null,
        slug
      })),
      title           : entry.title
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
          <span class="fixture-card-num">{{ row.num }}</span>
          <span class="fixture-card-title">{{ row.title }}</span>
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
              <a
                :href="`/rules/${seg.slug}`"
                class="composition-cards-chip"
                :data-family="seg.family"
                :title="seg.rule ? undefined : `${seg.slug} (${seg.family ?? 'undocumented'})`"
              >
                <span class="composition-cards-chip-label">
                  <span class="composition-cards-chip-slug">{{ seg.slug }}</span>
                </span>
              </a>
            </li>
          </ol>
          <div v-show="activeCase === row.case" class="composition-cards-toggle-slot" @click.stop>
            <FixtureToggle v-if="row.changesSource" v-model="activeTab" />
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
            <template v-if="row.descriptionHtml">
              <div class="fixture-card-desc" v-html="row.descriptionHtml" />
              <div class="fixture-card-rule" aria-hidden="true" />
            </template>
            <div v-if="activeCase === row.case" class="composition-cards-detail">
              <FixturePairDoc
                :active-tab="activeTab"
                :input-html="row.inputHtml"
                :output-html="row.outputHtml"
              />
            </div>
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
                <RuleTooltipPopper :rule="seg.rule">
                  <a
                    :href="`/rules/${seg.slug}`"
                    class="composition-cards-chip"
                    :data-family="seg.family"
                    :title="seg.rule ? undefined : `${seg.slug} (${seg.family ?? 'undocumented'})`"
                  >
                    <span class="composition-cards-chip-label">
                      <span class="composition-cards-chip-slug">{{ seg.slug }}</span>
                    </span>
                  </a>
                </RuleTooltipPopper>
              </li>
            </ol>
          </div>
        </div>
      </div>
    </li>
  </ol>
</template>
