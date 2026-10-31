<script setup lang="ts">
import { useData }  from 'vitepress'
import { computed } from 'vue'

import FixtureCard from '../fixtures/FixtureCard.vue'

import { pageExamples } from '../../../lib/rules/rule-view'

const props = defineProps<{ rule: string }>()

const { frontmatter } = useData()

const examples = computed(() => pageExamples(frontmatter.value, props.rule))
</script>

<template>
  <FixtureCard
    v-for="ex in examples"
    :key="ex.case"
    :rule="rule"
    :case="ex.case"
    :title-html="ex.titleHtml"
  />
</template>
