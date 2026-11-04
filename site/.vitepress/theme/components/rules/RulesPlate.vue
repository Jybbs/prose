<script setup lang="ts">
import FamilyChip from './FamilyChip.vue'
import NibBadge   from '../base/NibBadge.vue'

import { data as rules } from '../../../lib/rules/rules.data'
import { familyEmblem }  from '../../../lib/shared/emblems'
import { FAMILY_ORDER }  from '../../../lib/shared/registries'

const letters = Map.groupBy(rules.list, rule => rule.slug[0])
</script>

<template>
  <div class="isolate">
    <div class="rules-plate-key mt-24 flex flex-wrap gap-8 border-b-1 border-b-divider pb-18">
      <FamilyChip v-for="family in FAMILY_ORDER" :key="family" :family="family" />
    </div>
    <section v-for="[letter, group] in letters" :key="letter" class="rules-plate-letter grid grid-cols-3rem-minmax-1fr gap-x-14 gap-y-0 border-t-1 border-t-divider py-12 first-of-type:mt-18">
      <span class="rules-plate-cap font-display text-2-1rem leading-none font-500 text-text-3 italic" aria-hidden="true">{{ letter.toUpperCase() }}</span>
      <ul class="rules-plate-slugs m-0 list-none columns-2-11rem gap-x-24 p-0">
        <li
          v-for="rule in group"
          :key="rule.slug"
          class="m-0 flex break-inside-avoid items-center gap-8 py-3 focus-within:relative focus-within:z-20 hover:relative hover:z-20"
          :data-family="rule.family"
        >
          <NibBadge :emblem="familyEmblem(rule.family)" :label="rule.familyLabel" :size="14" />
          <RuleTooltipPopper :rule="rule">
            <a class="rules-plate-slug font-mono text-0-8rem leading-1-5 font-400 text-text-1 underline decoration-transparent underline-offset-3 transition-colors hover:text-brand-2 hover:decoration-family" :href="rule.href">{{ rule.slug }}</a>
          </RuleTooltipPopper>
        </li>
      </ul>
    </section>
  </div>
</template>
