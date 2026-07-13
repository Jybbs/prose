<script setup lang="ts">
import type { KeyedTokensInfo }    from '@shikijs/magic-move/types'
import { useIntersectionObserver } from '@vueuse/core'
import { computed, nextTick, onMounted, ref, shallowRef, useTemplateRef, watch } from 'vue'

import LintFlagPopper from '../rules/LintFlagPopper.vue'

import { useReducedMotion } from '../../../lib/composables/use-reduced-motion'
import { useSquiggleDraw }  from '../../../lib/composables/use-squiggle-draw'
import { magicMoveOptions, type MagicMovePanel } from '../../../lib/markdown/magic-move-options'
import type { FixtureTab }  from '../../../lib/shared/fixture-tab'
import { ruleDrawMs }       from '../../../lib/shared/paint'

const props = defineProps<{
  activeTab  : FixtureTab
  inputHtml  : string
  outputHtml : string
}>()

const reducedMotion = useReducedMotion()
const root          = useTemplateRef<HTMLElement>('root')
const popper        = useTemplateRef<InstanceType<typeof LintFlagPopper>>('popper')

const animate   = ref(false)
const animating = ref(false)
const duration  = ref(0)
const panel     = shallowRef<MagicMovePanel>(null)
const steps     = shallowRef<readonly KeyedTokensInfo[]>([])

const { drawSquiggles, undrawn } = useSquiggleDraw()

const activeHtml = computed(() => props.activeTab === 'before' ? props.inputHtml : props.outputHtml)
const step       = computed(() => props.activeTab === 'before' ? 0 : 1)

// The precompiled panel re-syncs its keys, with in-place side effects,
// whenever these prop identities change, so they stay stable across
// unrelated re-renders instead of rebuilding per template pass.
const morphOptions = computed(() => magicMoveOptions(duration.value))
const morphSteps   = computed(() => [...steps.value])

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
  steps.value    = await precompileMagicMove([before, after])
  duration.value = ruleDrawMs()
  panel.value    = ShikiMagicMovePrecompiled
  await nextTick()
  animate.value = true
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
      :steps="morphSteps"
      :step="step"
      :animate="animate && !reducedMotion"
      :options="morphOptions"
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
