<script setup lang="ts">
import { useHiddenTabindex } from '../../../lib/composables/use-aria-hidden'
import { data as rules }     from '../../../lib/rules/rules.data'
import { lookup }            from '../../../lib/shared/lookup'

const props = defineProps<{ slug : string }>()

const tabindex = useHiddenTabindex()
const rule     = lookup(rules.bySlug, props.slug, 'Inline rule link')
</script>

<template>
  <a
    class="rule-link"
    :data-family="rule.family"
    :data-rule="slug"
    :href="rule.href"
    :tabindex="tabindex"
  >{{ slug }}</a>
</template>
