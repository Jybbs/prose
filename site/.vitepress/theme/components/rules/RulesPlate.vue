<script setup lang="ts">
import NibBadge from '../base/NibBadge.vue'

import { useRaisedFamily } from '../../../lib/composables/use-raised-family'
import { data as rules }   from '../../../lib/rules/rules.data'
import { familyEmblem }    from '../../../lib/shared/emblems'
import { FAMILY_ORDER }    from '../../../lib/shared/registries'

const KEY     = FAMILY_ORDER.map(family => ({ emblem: familyEmblem(family), family }))
const letters = Map.groupBy(rules.list, rule => rule.slug[0])

const { on, pinned, shown } = useRaisedFamily()
</script>

<template>
  <div class="rules-plate" :data-raised="shown">
    <div class="rules-plate-key" role="group" aria-label="Rule families">
      <button
        v-for="{ emblem, family } in KEY"
        :key="family"
        type="button"
        class="family-chip"
        :class="{ 'is-raised': family === shown }"
        :data-family="family"
        :aria-pressed="family === pinned"
        v-bind="on(family)"
      >
        <NibBadge :emblem="emblem" :size="15.2" />
        <span>{{ emblem.name }}</span>
      </button>
    </div>
    <section v-for="[letter, group] in letters" :key="letter" class="rules-plate-letter">
      <span class="rules-plate-cap" aria-hidden="true">{{ letter.toUpperCase() }}</span>
      <ul class="rules-plate-slugs">
        <li
          v-for="rule in group"
          :key="rule.slug"
          :class="{ 'is-raised': rule.family === shown }"
          :data-family="rule.family"
          v-bind="on(rule.family)"
        >
          <NibBadge :emblem="familyEmblem(rule.family)" :label="rule.familyLabel" :size="14" />
          <RuleTooltipPopper :rule="rule">
            <a class="rules-plate-slug" :href="rule.href">{{ rule.slug }}</a>
          </RuleTooltipPopper>
        </li>
      </ul>
    </section>
  </div>
</template>
