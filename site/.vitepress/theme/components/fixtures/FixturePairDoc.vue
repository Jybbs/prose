<script setup lang="ts">
import { useIntersectionObserver }                                   from '@vueuse/core'
import { computed, nextTick, onMounted, ref, useTemplateRef, watch } from 'vue'

import CopyButton        from '../base/CopyButton.vue'
import LintFlagPopper    from '../rules/LintFlagPopper.vue'
import SandboxSeedButton from '../sandbox/SandboxSeedButton.vue'

import { useMagicMove }      from '../../../lib/composables/use-magic-move'
import { useReducedMotion }  from '../../../lib/composables/use-reduced-motion'
import { useSquiggleDraw }   from '../../../lib/composables/use-squiggle-draw'
import type { SavedSession } from '../../../lib/sandbox/session'
import type { FixtureTab }   from '../../../lib/shared/fixture-tab'

const props = defineProps<{
  activeTab    : FixtureTab
  inputHtml    : string
  outputHtml   : string
  sandboxSeed ?: SavedSession
}>()

const reducedMotion = useReducedMotion()
const root          = useTemplateRef<HTMLElement>('root')
const popper        = useTemplateRef<InstanceType<typeof LintFlagPopper>>('popper')

const animate   = ref(false)
const animating = ref(false)

const { drawSquiggles, undrawn } = useSquiggleDraw()

const { morphOptions, morphSteps, panel, precompile, steps } = useMagicMove()

const activeHtml = computed(() => props.activeTab === 'before' ? props.inputHtml : props.outputHtml)
const step       = computed(() => props.activeTab === 'before' ? 0 : 1)

// The copy button and lang chip render as the component's own persistent
// elements so they hold their place across the morph, and the static markup
// drops them so the two do not double up. The source resolves client-side,
// where `codeFrom` has a DOM to read.
const activeCode   = computed(() => typeof window === 'undefined' ? '' : codeFrom(activeHtml.value))
const strippedHtml = computed(() => activeHtml.value
  .replace(/<button\b[^>]*class="copy"[^>]*>\s*<\/button>/, '')
  .replace(/<span class="lang">[^<]*<\/span>/, ''))

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
  steps.value = await precompile(before, after)
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
  <div ref="root" class="fixture-pair fixture-pair-doc panel panel-clip copy-host">
    <component
      :is="panel"
      v-if="panel && morphSteps.length"
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
      v-html="strippedHtml"
    />
    <CopyButton :source="activeCode" label="Copy code" />
    <SandboxSeedButton v-if="sandboxSeed" :seed="sandboxSeed" />
    <span class="lang">python</span>
    <LintFlagPopper ref="popper" />
  </div>
</template>
