<script setup lang="ts">
import { useHiddenTabindex } from '../../../../lib/composables/use-aria-hidden'
import type { RenderedRule } from '../../../../lib/rules/rules.data'
import { familyEmblem }      from '../../../../lib/shared/emblems'
import NibBadge              from '../../base/NibBadge.vue'
import RuleTooltipPopper     from '../../rules/RuleTooltipPopper.vue'

defineProps<{ rule: RenderedRule | undefined, swap: string }>()

const tabindex = useHiddenTabindex()
</script>

<template>
  <RuleTooltipPopper v-if="rule" :rule="rule">
    <a
      class="rule-chip surface-rail-chip"
      :href="rule.href"
      :data-family="rule.family"
      :tabindex="tabindex"
    >
      <NibBadge :emblem="familyEmblem(rule.family)" :size="14" />
      <Transition :name="swap" mode="out-in">
        <span :key="rule.slug" class="rule-chip-slug">{{ rule.slug }}</span>
      </Transition>
    </a>
  </RuleTooltipPopper>
</template>
