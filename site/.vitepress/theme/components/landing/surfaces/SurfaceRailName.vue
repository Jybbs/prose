<script setup lang="ts">
import { useHiddenTabindex } from '../../../../lib/composables/use-aria-hidden'
import type { RenderedRule } from '../../../../lib/rules/rules.data'
import { familyEmblem }      from '../../../../lib/shared/emblems'
import { NibBadge }          from '../../../../kit'
import RuleTooltipPopup      from '../../rules/RuleTooltipPopup.vue'

defineProps<{ rule: RenderedRule | undefined, swap: string }>()

const tabindex = useHiddenTabindex()
</script>

<template>
  <RuleTooltipPopup v-if="rule" :rule="rule">
    <a
      class="surface-rail-chip chip rule-chip"
      :href="rule.href"
      :data-family="rule.family"
      :tabindex="tabindex"
    >
      <NibBadge :emblem="familyEmblem(rule.family)" :size="14" />
      <Transition :name="swap" mode="out-in">
        <span :key="rule.slug" class="font-mono">{{ rule.slug }}</span>
      </Transition>
    </a>
  </RuleTooltipPopup>
</template>
