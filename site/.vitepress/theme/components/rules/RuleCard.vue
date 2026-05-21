<script setup lang="ts">
import { CATEGORY_META, FAMILY_META } from '../../../lib/shared/registries'
import type { RenderedRule }          from '../../../data/rules.data'

const props = defineProps<{
  index : number
  rule  : RenderedRule
}>()

function toTitle(slug: string): string {
  return slug.split('-').map(p => p.charAt(0).toUpperCase() + p.slice(1)).join('-')
}

function toRoman(n: number): string {
  const map: Array<[number, string]> = [
    [1000, 'M'], [900, 'CM'], [500, 'D'], [400, 'CD'],
    [100,  'C'], [90,  'XC'], [50,  'L'], [40,  'XL'],
    [10,   'X'], [9,   'IX'], [5,   'V'], [4,   'IV'],
    [1,    'I']
  ]
  let out = ''
  for (const [v, s] of map) {
    while (n >= v) { out += s; n -= v }
  }
  return out
}
</script>

<template>
  <article
    class="rule-card"
    :data-category="rule.category"
    :data-family="rule.family"
  >
    <a
      class="rule-card-cover"
      :href="`/rules/${rule.slug}`"
      :aria-label="toTitle(rule.slug)"
    />
    <header class="rule-card-header">
      <span class="rule-card-folio" aria-hidden="true">{{ toRoman(index + 1) }}</span>
      <h3 class="rule-card-name">{{ toTitle(rule.slug) }}</h3>
      <a
        class="rule-card-circle rule-card-circle-category"
        :href="`/rules/${rule.category}/`"
        :aria-label="`See all ${CATEGORY_META[rule.category].label.toLowerCase()} rules`"
      >
        <span aria-hidden="true">{{ CATEGORY_META[rule.category].badge }}</span>
      </a>
      <a
        class="rule-card-circle rule-card-circle-family"
        :href="`/rules/${rule.family}/`"
        :aria-label="`See all ${FAMILY_META[rule.family].label.toLowerCase()} rules`"
      >
        <span aria-hidden="true">{{ FAMILY_META[rule.family].badge }}</span>
      </a>
    </header>
    <p class="rule-card-caption" v-html="rule.captionHtml"></p>
  </article>
</template>
