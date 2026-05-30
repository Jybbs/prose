<script setup lang="ts">
import { useIntersectionObserver, useMediaQuery } from '@vueuse/core'
import type { KeyedTokensInfo }                   from 'shiki-magic-move/types'
import { computed, nextTick, ref, shallowRef, useTemplateRef } from 'vue'
import type { Component }                         from 'vue'

import type { FixtureTab } from '../../../lib/shared/fixture-tab'

const props = defineProps<{
  activeTab  : FixtureTab
  inputHtml  : string
  outputHtml : string
}>()

const reducedMotion = useMediaQuery('(prefers-reduced-motion: reduce)')
const root          = useTemplateRef<HTMLElement>('root')

const animate  = ref(false)
const duration = ref(0)
const panel    = shallowRef<Component | null>(null)
const steps    = shallowRef<readonly KeyedTokensInfo[]>([])

const activeHtml = computed(() => props.activeTab === 'before' ? props.inputHtml : props.outputHtml)
const step       = computed(() => props.activeTab === 'before' ? 0 : 1)

// Recover the source from a prebuilt highlight, reading only `<pre><code>` so
// the lang chip and copy button stay out of the retokenized code.
function codeFrom(html: string): string {
  return new DOMParser().parseFromString(html, 'text/html')
    .querySelector('pre code')?.textContent?.replace(/\s+$/, '') ?? ''
}

// Once the fixture scrolls into view, load the renderer and highlighter,
// paint the active step, then enable motion for later toggles. The
// `.fixture-card-rule` draw and the move share `--prose-rule-draw-ms`.
async function prepare(): Promise<void> {
  if (panel.value || reducedMotion.value) return
  const before = codeFrom(props.inputHtml)
  const after  = codeFrom(props.outputHtml)
  if (before === after) return
  const [{ precompileMagicMove }, { ShikiMagicMovePrecompiled }] = await Promise.all([
    import('../../../lib/markdown/magic-move'),
    import('shiki-magic-move/vue')
  ])
  const rootStyle = getComputedStyle(document.documentElement)
  steps.value    = await precompileMagicMove([before, after])
  duration.value = Number(rootStyle.getPropertyValue('--prose-rule-draw-ms'))
  panel.value    = ShikiMagicMovePrecompiled
  await nextTick()
  animate.value = true
}

const { stop } = useIntersectionObserver(root, ([entry]) => {
  if (entry.isIntersecting) { prepare(); stop() }
})
</script>

<template>
  <div ref="root" class="fixture-pair fixture-pair-doc">
    <component
      :is="panel"
      v-if="panel"
      class="fixture-pair-panel"
      :steps="steps"
      :step="step"
      :animate="animate && !reducedMotion"
      :options="{ containerStyle: false, delayMove: 0, duration, stagger: 3 }"
    />
    <div v-else class="fixture-pair-panel" v-html="activeHtml" />
  </div>
</template>
