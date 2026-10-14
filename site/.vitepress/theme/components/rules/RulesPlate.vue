<script setup lang="ts">
import FamilyChip from './FamilyChip.vue'
import NibBadge   from '../base/NibBadge.vue'

import { data as rules } from '../../../lib/rules/rules.data'
import { familyEmblem }  from '../../../lib/shared/emblems'
import { FAMILY_ORDER }  from '../../../lib/shared/registries'

const letters = Map.groupBy(rules.list, rule => rule.slug[0])
</script>

<template>
  <div class="rules-plate">
    <div class="rules-plate-key">
      <FamilyChip v-for="family in FAMILY_ORDER" :key="family" :family="family" />
    </div>
    <section v-for="[letter, group] in letters" :key="letter" class="rules-plate-letter">
      <span class="rules-plate-cap" aria-hidden="true">{{ letter.toUpperCase() }}</span>
      <ul class="rules-plate-slugs">
        <li v-for="rule in group" :key="rule.slug" :data-family="rule.family">
          <NibBadge :emblem="familyEmblem(rule.family)" :label="rule.familyLabel" :size="14" />
          <RuleTooltipPopper :rule="rule">
            <a class="rules-plate-slug" :href="rule.href">{{ rule.slug }}</a>
          </RuleTooltipPopper>
        </li>
      </ul>
    </section>
  </div>
</template>
