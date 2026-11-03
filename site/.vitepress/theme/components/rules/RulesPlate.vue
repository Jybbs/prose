<script setup lang="ts">
import { computed, nextTick, shallowRef, useTemplateRef, watch } from 'vue'

import NibBadge          from '../base/NibBadge.vue'
import RuleTooltipPopper from './RuleTooltipPopper.vue'

import { useRaisedFamily }       from '../../../lib/composables/use-raised-family'
import { glide }                 from '../../../lib/rules/plate-glide'
import { data as rules }         from '../../../lib/rules/rules.data'
import { arrowIndex }            from '../../../lib/shared/arrow-keys'
import { familyEmblem }          from '../../../lib/shared/emblems'
import { morphEase, ruleDrawMs } from '../../../lib/shared/paint'
import { FAMILY_ORDER }          from '../../../lib/shared/registries'
import type { RuleFamily }       from '../../../lib/shared/registries'

const KEY = FAMILY_ORDER.map(family => ({
  count  : rules.list.filter(rule => rule.family === family).length,
  emblem : familyEmblem(family),
  family : family
}))

const plate = useTemplateRef<HTMLElement>('plate')

const { on, pinned, release, shown } = useRaisedFamily()

// Holds the family whose rules the list shows gathered above the letters,
// which follows the pinned family once each move has measured the list.
const arranged = shallowRef<RuleFamily | null>(null)
const roving   = shallowRef<RuleFamily>(FAMILY_ORDER[0])
const spoken   = shallowRef('')

const counted = computed(() => KEY.find(({ family }) => family === shown.value))

// Lists the group of the arranged family's rules above the letters, each
// letter holding the rest of its rules.
const sections = computed(() => {
  const held    = arranged.value
  const letters = [...Map.groupBy(rules.list, rule => rule.slug[0]!)].map(([key, group]) =>
    ({ family: null, key, rules: group.filter(rule => rule.family !== held) }))
  if (!held) return letters
  return [{ family: held, key: 'gathered', rules: rules.list.filter(rule => rule.family === held) }, ...letters]
})

watch(pinned, family => {
  const entry = KEY.find(each => each.family === family)
  spoken.value = entry ? `${entry.count} ${entry.emblem.name} rules moved above the list.` : 'All families shown.'
  void glide(plate.value!, async () => {
    arranged.value = family as RuleFamily | null
    await nextTick()
  }, { duration: ruleDrawMs(), easing: morphEase() })
})

// Moves the focus and the key's one tab stop to the chip at `at`.
function focusChip(at: number): void {
  roving.value = KEY[at]!.family
  plate.value!.querySelectorAll<HTMLElement>('.rules-plate-key button')[at]!.focus()
}

// Moves the focus across the key's chips under the arrow keys, Home, and
// End, wrapping past either end.
function move(event: KeyboardEvent, at: number): void {
  const to = arrowIndex(event.key, at, KEY.length, true)
  if (to === undefined) return
  event.preventDefault()
  focusChip(to)
}

// Returns every row to its letter and the focus to the chip that held them.
function releaseAll(): void {
  const back = KEY.findIndex(({ family }) => family === pinned.value)
  release()
  focusChip(Math.max(back, 0))
}
</script>

<template>
  <div ref="plate" class="rules-plate">
    <div class="rules-plate-key" role="toolbar" aria-label="Rule families">
      <span v-for="({ emblem, family }, at) in KEY" :key="family" class="rules-plate-split" :data-family="family">
        <button
          type="button"
          class="family-chip nib-ground"
          :aria-pressed="family === pinned"
          :tabindex="family === roving ? 0 : -1"
          v-bind="on(family)"
          @click="roving = family"
          @keydown="move($event, at)"
        >
          <NibBadge :emblem="emblem" :size="15.2" />
          <span>{{ emblem.name }}</span>
        </button>
        <a class="rules-plate-go" :href="`/rules/${family}/`" :aria-label="`${emblem.name} page`">
          <span class="nib-stroke is-quiet" aria-hidden="true">→</span>
        </a>
      </span>
    </div>
    <div class="rules-plate-census-row">
      <div class="rules-plate-census" aria-hidden="true">
        <span
          v-for="{ count, family } in KEY"
          :key="family"
          :class="{ 'is-raised': family === shown }"
          :data-family="family"
          :style="{ flex: `${count} 1 0` }"
        ></span>
      </div>
      <span class="rules-plate-census-cap">
        {{ counted ? `${counted.count} of ${rules.list.length}` : rules.list.length }} rules
      </span>
      <button type="button" class="rules-plate-all" :class="{ 'is-idle': !pinned }" @click="releaseAll">
        All families
      </button>
    </div>
    <p class="rules-plate-live" aria-live="polite">{{ spoken }}</p>
    <section
      v-for="section in sections"
      :key="section.key"
      class="rules-plate-letter"
      :class="{ 'is-out': !section.rules.length, 'rules-plate-gathered': section.family }"
      :data-letter="section.key"
    >
      <span class="rules-plate-cap" :data-family="section.family ?? undefined" aria-hidden="true">
        <NibBadge v-if="section.family" :emblem="familyEmblem(section.family)" :size="28" />
        <template v-else>{{ section.key.toUpperCase() }}</template>
      </span>
      <ul class="rules-plate-slugs" :aria-label="section.family ? `${familyEmblem(section.family).name} rules` : undefined">
        <li v-for="rule in section.rules" :key="rule.slug" :data-slug="rule.slug" :data-family="rule.family">
          <NibBadge :emblem="familyEmblem(rule.family)" :label="rule.familyLabel" :size="14" />
          <RuleTooltipPopper :rule="rule">
            <a class="rules-plate-slug nib-stroke is-quiet" :href="rule.href">{{ rule.slug }}</a>
          </RuleTooltipPopper>
        </li>
      </ul>
    </section>
  </div>
</template>
