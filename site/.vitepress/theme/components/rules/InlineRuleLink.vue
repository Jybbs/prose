<script setup lang="ts">
import { useAriaHidden }  from '../../../lib/composables/use-aria-hidden'
import { data as rules }  from '../../../lib/rules/rules.data'
import { lookup }         from '../../../lib/shared/lookup'

const props = defineProps<{ slug : string }>()

const ariaHidden = useAriaHidden()
const rule       = lookup(rules.bySlug, props.slug, 'Inline rule link')
</script>

<template>
  <RuleTooltipPopper :rule="rule">
    <a
      class="rule-link"
      :data-family="rule.family"
      :href="rule.href"
      :tabindex="ariaHidden ? -1 : undefined"
    >{{ slug }}</a>
  </RuleTooltipPopper>
</template>
