<script setup lang="ts">
import { useIntersectionObserver } from '@vueuse/core'
import { PopperWrapper }           from 'floating-vue'
import type { KeyedTokensInfo }    from '@shikijs/magic-move/types'
import { computed, nextTick, onMounted, ref, shallowRef, useTemplateRef, watch } from 'vue'

import RuleCard from '../rules/RuleCard.vue'

import { data as rules }     from '../../../lib/rules/rules.data'
import type { RenderedRule } from '../../../lib/rules/rules.data'
import { useReducedMotion }  from '../../../lib/composables/use-reduced-motion'
import { lintShorthand }     from '../../../lib/fixtures/lint-shorthand'
import type { Shorthand }    from '../../../lib/fixtures/lint-shorthand'
import type { FixtureTab }   from '../../../lib/shared/fixture-tab'
import { inlineCode }        from '../../../lib/shared/inline-code'

const props = defineProps<{
  activeTab  : FixtureTab
  inputHtml  : string
  outputHtml : string
}>()

interface ActiveFinding {
  message   : string
  rule      : RenderedRule
  shorthand : Shorthand | null
}

const reducedMotion = useReducedMotion()
const root          = useTemplateRef<HTMLElement>('root')

type Panel = typeof import('@shikijs/magic-move/vue').ShikiMagicMovePrecompiled | null

const animate   = ref(false)
const animating = ref(false)
const duration  = ref(0)
const panel     = shallowRef<Panel>(null)
const steps     = shallowRef<readonly KeyedTokensInfo[]>([])
const undrawn   = ref(false)

const active      = ref<ActiveFinding | null>(null)
const activeFlag  = shallowRef<HTMLElement | null>(null)
const messageHtml = computed(() => inlineCode(active.value?.message ?? ''))

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

// The `.lint-flag` anchors sit inside `v-html` static HTML, so event
// delegation finds them and the popper takes the hovered flag as a dynamic
// reference node rather than wrapping each anchor in a component.
function show(event: Event): void {
  const flag = (event.target as HTMLElement).closest<HTMLElement>('.lint-flag')
  const rule = flag?.dataset.rule ? rules.bySlug[flag.dataset.rule] : undefined
  if (!flag || !rule) return
  const message    = flag.dataset.message ?? ''
  activeFlag.value = flag
  active.value = {
    message,
    rule,
    shorthand : lintShorthand({
      before    : flag.dataset.before,
      flagged   : flag.textContent ?? '',
      message,
      rule      : flag.dataset.rule ?? '',
      suggested : flag.dataset.suggested
    })
  }
}

function hide(): void {
  active.value = null
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
      @mouseover="show"
      @mouseout="hide"
      @focusin="show"
      @focusout="hide"
      v-html="activeHtml"
    />
    <PopperWrapper
      theme="rule-card"
      placement="bottom-start"
      popper-class="lint-popover fam-lint"
      :auto-hide="false"
      :distance="6"
      :handle-resize="false"
      :popper-triggers="[]"
      :reference-node="() => activeFlag!"
      :shown="active !== null"
      :triggers="[]"
    >
      <template #popper>
        <RuleCard v-if="active" :rule="active.rule" :clickable="false">
          <template #header>
            <span v-if="active.shorthand?.kind === 'replace'" class="lint-shorthand">
              <span class="lint-chip lint-chip-struck">{{ active.shorthand.before }}</span>
              <span class="lint-into" aria-hidden="true">→</span>
              <span class="lint-chip lint-chip-suggest">{{ active.shorthand.after }}</span>
            </span>
            <span v-else-if="active.shorthand?.kind === 'remove'" class="lint-shorthand">
              <span class="lint-chip lint-chip-struck">{{ active.shorthand.text }}</span>
            </span>
            <span v-else class="lint-message" v-html="messageHtml" />
          </template>
        </RuleCard>
      </template>
    </PopperWrapper>
  </div>
</template>
