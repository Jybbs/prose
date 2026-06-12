<script setup lang="ts">
import { ref }              from 'vue'

import MiddleEllipsis        from '../base/MiddleEllipsis.vue'
import { useMeasuredCssVar } from './use-measured-css-var'

import type { RenderedRule } from '../../../data/rules.data'

const props = withDefaults(
  defineProps<{
    clickable ?: boolean
    rule       : RenderedRule
  }>(),
  { clickable : true }
)

const article = ref<HTMLElement | null>(null)
const caption = ref<HTMLElement | null>(null)

if (!props.clickable) {
  useMeasuredCssVar({
    measure  : () => {
      const c = caption.value
      if (!c) return null
      const range = document.createRange()
      range.selectNodeContents(c)
      const lines = new Map<number, { left: number, right: number }>()
      for (const rect of range.getClientRects()) {
        const top   = Math.round(rect.top)
        const entry = lines.get(top) ?? { left: Infinity, right: -Infinity }
        entry.left  = Math.min(entry.left,  rect.left)
        entry.right = Math.max(entry.right, rect.right)
        lines.set(top, entry)
      }
      let widest = 0
      for (const { left, right } of lines.values()) {
        widest = Math.max(widest, right - left)
      }
      return widest > 0 ? widest : null
    },
    propName : '--rule-card-caption-width',
    target   : article,
    triggers : [() => props.rule.slug]
  })
}
</script>

<template>
  <article
    ref="article"
    class="rule-card"
    :data-category="rule.category"
    :data-family="rule.family"
  >
    <a v-if="clickable" class="rule-card-cover" :href="rule.href" :aria-label="rule.name" />
    <div v-if="$slots.header" class="rule-card-header"><slot name="header" /></div>
    <span
      class="rule-card-badge"
      :title="rule.familyLabel"
      aria-hidden="true"
    >{{ rule.familyBadge }}</span>
    <div class="rule-card-name">
      <h3 class="rule-card-slug" :title="rule.slug">
        <MiddleEllipsis :text="rule.slug" />
      </h3>
      <span class="rule-card-fam">{{ rule.familyLabel }}</span>
    </div>
    <p ref="caption" class="rule-card-caption" v-html="rule.captionHtml"></p>
  </article>
</template>
