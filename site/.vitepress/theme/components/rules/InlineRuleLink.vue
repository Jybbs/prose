<script setup lang="ts">
import { data as rules } from '../../../data/rules.data'
import { lookup }        from '../../../lib/shared/lookup'

const props = defineProps<{ slug : string }>()

const rule = lookup(rules.bySlug, props.slug, 'Inline rule link')
</script>

<template>
  <RuleTooltipPopper :rule="rule">
    <a class="rule-link" :data-family="rule.family" :href="`/rules/${slug}`">{{ slug }}</a>
  </RuleTooltipPopper>
</template>
