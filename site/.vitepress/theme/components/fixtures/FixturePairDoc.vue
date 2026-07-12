<script setup lang="ts">
import { useIntersectionObserver } from '@vueuse/core'
import type { KeyedTokensInfo }    from '@shikijs/magic-move/types'
import { computed, nextTick, onMounted, ref, shallowRef, useTemplateRef, watch } from 'vue'

import LintFlagPopper from '../rules/LintFlagPopper.vue'

import { useReducedMotion }  from '../../../lib/composables/use-reduced-motion'
import type { FixtureTab }   from '../../../lib/shared/fixture-tab'

const props = defineProps<{
  activeTab  : FixtureTab
  inputHtml  : string
  outputHtml : string
}>()

const reducedMotion = useReducedMotion()
const root          = useTemplateRef<HTMLElement>('root')
const popper        = useTemplateRef<InstanceType<typeof LintFlagPopper>>('popper')

type Panel = typeof import('@shikijs/magic-move/vue').ShikiMagicMovePrecompiled | null

const animate   = ref(false)
const animating = ref(false)
const duration  = ref(0)
const panel     = shallowRef<Panel>(null)
const steps     = shallowRef<readonly KeyedTokensInfo[]>([])
const undrawn   = ref(false)

const activeHtml = computed(() => props.activeTab === 'before' ? props.inputHtml : props.outputHtml)
const step       = computed(() => props.activeTab === 'before' ? 0 : 1)

// Recover the source from a prebuilt highlight, reading only `<pre><code>` so
// the lang chip and copy button stay out of the retokenized code.
function codeFrom(html: string): string {
  return new DOMParser().parseFromString(html, 'text/html')
    .querySelector('pre code')?.textContent?.trimEnd() ?? ''
}

// Once the fixture scrolls into view, load the renderer and highlighter,
// paint the active step, then enable motion for later toggles. The
// `.fixture-card-rule` draw, the move, and the squiggle draw share
// `--prose-rule-draw-ms`.
async function prepare(): Promise<void> {
  if (panel.value || reducedMotion.value) return
  const before = codeFrom(props.inputHtml)
  const after  = codeFrom(props.outputHtml)
  if (before === after) return
  const [{ precompileMagicMove }, { ShikiMagicMovePrecompiled }] = await Promise.all([
    import('../../../lib/markdown/magic-move'),
    import('@shikijs/magic-move/vue')
  ])
  const rootStyle = getComputedStyle(document.documentElement)
  steps.value    = await precompileMagicMove([before, after])
  duration.value = Number(rootStyle.getPropertyValue('--prose-rule-draw-ms'))
  panel.value    = ShikiMagicMovePrecompiled
  await nextTick()
  animate.value = true
}

// Replays the left-to-right squiggle draw, staging `lint-undrawn` and
// lifting it two frames later so the CSS transition re-fires.
function drawSquiggles(): void {
  if (typeof requestAnimationFrame === 'undefined') return
  undrawn.value = true
  requestAnimationFrame(() => requestAnimationFrame(() => { undrawn.value = false }))
}

// Magic-move owns the panel through the morph, and on settle the
// decorated static panel returns so its `.lint-flag` hovers work and the
// squiggles draw back in.
function settle(): void {
  animating.value = false
  drawSquiggles()
}

// Hand the panel to magic-move the instant the side flips, before its
// deferred render measures, so the morph is never sized while hidden.
// With no morph to run (a lint-only fixture or reduced motion), draw the
// squiggles directly so the line lands the same way it does after a morph.
watch(() => props.activeTab, tab => {
  if (panel.value && animate.value && !reducedMotion.value) {
    animating.value = true
  } else if (tab === 'after') {
    drawSquiggles()
  }
})

// The static HTML ships the wave drawn, so staging it undrawn at mount,
// before the first paint, keeps the first-view entrance a clean draw
// rather than a shrink-then-grow on intersection.
onMounted(() => { undrawn.value = true })

const { stop } = useIntersectionObserver(root, ([entry]) => {
  if (!entry.isIntersecting) return
  prepare()
  drawSquiggles()
  stop()
})
</script>

<template>
  <div ref="root" class="fixture-pair fixture-pair-doc panel panel-clip">
    <component
      :is="panel"
      v-if="panel"
      v-show="animating"
      class="fixture-pair-panel"
      :steps="[...steps]"
      :step="step"
      :animate="animate && !reducedMotion"
      :options="{ containerStyle: false, delayMove: 0, duration, stagger: 3 }"
      @end="settle"
    />
    <div
      v-show="!animating"
      class="fixture-pair-panel"
      :class="{ 'lint-undrawn': undrawn }"
      @mouseover="popper?.show"
      @mouseout="popper?.hide"
      @focusin="popper?.show"
      @focusout="popper?.hide"
      v-html="activeHtml"
    />
    <LintFlagPopper ref="popper" />
  </div>
</template>
