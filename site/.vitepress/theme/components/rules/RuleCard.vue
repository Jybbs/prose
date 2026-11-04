<script setup lang="ts">
import { ref } from 'vue'

import MiddleEllipsis from '../base/MiddleEllipsis.vue'
import NibBadge       from '../base/NibBadge.vue'

import { useMeasuredCssVar } from '../../../lib/composables/use-measured-css-var'
import type { RenderedRule } from '../../../lib/rules/rules.data'
import { familyEmblem }      from '../../../lib/shared/emblems'

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
    measure: () => {
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
  <article ref="article" class="rule-card" :data-family="rule.family">
    <a v-if="clickable" class="rule-card-cover" :href="rule.href" :aria-label="rule.name" />
    <div v-if="$slots.header" class="rule-card-header pointer-events-none relative z-2"><slot name="header" /></div>
    <span class="rule-card-mark pointer-events-none relative z-2 flex items-center justify-center border-l-2 border-l-family ground py-12">
      <NibBadge :emblem="familyEmblem(rule.family)" :label="rule.familyLabel" :size="18" />
    </span>
    <div class="rule-card-body pointer-events-none relative z-2 grid min-w-0 content-center gap-6 bg-family-wash-fade px-20 pt-14 pb-15 max-mobile:px-14">
      <h3 class="rule-card-slug m-0 min-w-0 border-none font-mono text-0-92rem leading-1-3 font-600 -tracking-0-01em text-text-1" :title="rule.slug">
        <MiddleEllipsis :text="rule.slug" />
      </h3>
      <p ref="caption" class="rule-card-caption" v-html="rule.captionHtml"></p>
    </div>
  </article>
</template>
