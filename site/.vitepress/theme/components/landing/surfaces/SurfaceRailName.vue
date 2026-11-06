<script setup lang="ts">
import { useHiddenTabindex } from '../../../../lib/composables/use-aria-hidden'
import type { RenderedRule } from '../../../../lib/rules/rules.data'
import { familyEmblem }      from '../../../../lib/shared/emblems'
import { NibBadge }          from '../../../../kit'

defineProps<{ rule: RenderedRule | undefined, swap: string }>()

const tabindex = useHiddenTabindex()
</script>

<template>
  <a
    v-if="rule"
    class="surface-rail-chip chip rule-chip"
    :data-family="rule.family"
    :data-rule="rule.slug"
    :href="rule.href"
    :tabindex="tabindex"
  >
    <NibBadge :emblem="familyEmblem(rule.family)" :size="14" />
    <Transition :name="swap" mode="out-in">
      <span :key="rule.slug" class="font-mono">{{ rule.slug }}</span>
    </Transition>
  </a>
</template>
