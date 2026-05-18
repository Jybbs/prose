<script setup lang="ts">
import { useData }  from 'vitepress'
import { computed } from 'vue'

import Kicker   from '../base/Kicker.vue'
import RuleChip from '../rules/RuleChip.vue'

import { data as rules }  from '../../../data/rules.data'
import { useCurrentRule } from '../../../lib/shared/route'

const current        = useCurrentRule()
const { frontmatter } = useData()

const related = computed(() => {
  if (!current.value) return []
  const slugs = frontmatter.value.related as string[] | undefined
  if (slugs?.length) {
    return slugs
      .map(slug => rules.bySlug[slug])
      .filter((r): r is NonNullable<typeof r> => r !== undefined)
  }
  return rules.list
    .filter(r => r.slug !== current.value!.slug && r.category === current.value!.category)
    .slice(0, 5)
})
</script>

<template>
  <div v-if="current && related.length" class="related-rules">
    <Kicker class="related-rules-kicker">Related</Kicker>
    <ul class="related-rules-list">
      <li v-for="rule in related" :key="rule.slug">
        <RuleChip :slug="rule.slug" />
      </li>
    </ul>
  </div>
</template>
