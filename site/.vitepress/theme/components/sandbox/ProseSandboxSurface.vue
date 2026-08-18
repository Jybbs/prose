<script setup lang="ts">
import { promiseTimeout, useTimeoutFn }                              from '@vueuse/core'
import { computed, nextTick, onMounted, ref, useTemplateRef, watch } from 'vue'

import LintFlagPopper    from '../rules/LintFlagPopper.vue'
import SandboxCodeEditor from './SandboxCodeEditor.vue'

import { useMagicMove }          from '../../../lib/composables/use-magic-move'
import type { ProseSandbox }     from '../../../lib/composables/use-prose-sandbox'
import { useReducedMotion }      from '../../../lib/composables/use-reduced-motion'
import { useSquiggleDraw }       from '../../../lib/composables/use-squiggle-draw'
import { lintDecorations }       from '../../../lib/markdown/lint-decorations'
import * as magicMoveDelta       from '../../../lib/markdown/magic-move-delta'
import { REPO_URL }              from '../../../lib/shared/constants'
import { highlight }             from '../../../lib/shared/highlight'
import { latestRun }             from '../../../lib/shared/latest-run'
import { externalAttrs }         from '../../../lib/shared/links'
import { nextPaint, ruleDrawMs } from '../../../lib/shared/paint'

const NO_PAIR = { churn: 0, lines: [0, 0] as const, shifts: 0 }

const props   = defineProps<{ guide?: number | null, guideHue?: string, sandbox: ProseSandbox }>()
const editing = defineModel<boolean>('editing', { default: false })
const { diagnostics, drawn, error, formatted, source, unstable } = props.sandbox

const reportUrl = `${REPO_URL}/issues/new?template=unstable-output.yml`

const reducedMotion = useReducedMotion()
const display       = useTemplateRef<HTMLElement>('display')
const editor        = useTemplateRef<InstanceType<typeof SandboxCodeEditor>>('editor')
const popper        = useTemplateRef<InstanceType<typeof LintFlagPopper>>('popper')
const surface       = useTemplateRef<HTMLElement>('surface')

const animate     = ref(false)
const displayHtml = ref('')
const draft       = ref('')
const morphing    = ref(false)
const restHeight  = ref(0)
const step        = ref(0)

const { drawSquiggles, undrawn } = useSquiggleDraw()

const { morphOptions, morphSteps, panel, precompile, steps, watchdogMs } = useMagicMove(0)

// Revealing the panel trades a laid-out display for one that has to lay out
// from `display: none`, so the section would drop to the surfaces grid's
// `min-height` in between. The outgoing height floors the section until the
// step flips, from where the container's own height animation drives it.
const heldHeight = computed(() =>
  morphing.value && step.value === 0 ? `${restHeight.value}px` : undefined)

const ruleCodes = computed(() => new Set(diagnostics.value.map(finding => finding.code)))

const watchdog = useTimeoutFn(
  () => {
    if (!morphing.value || step.value === 0) return
    pendingEnds = 0
    settleMorph()
  },
  () => watchdogMs.value,
  { immediate: false }
)

const run = latestRun()

let pendingEnds = 0
let previous    = ''
let seenDraw    = drawn.value
let shownRules  = new Set<string>()

// Renders the settled output as highlighted HTML carrying the lint
// squiggles, then morphs from the prior output when motion is allowed and
// the surface is not mid-edit. One magic-move panel serves every
// transition, swapped to the incoming rest state before the step flips.
async function render(next: string): Promise<void> {
  const superseded = run.begin()
  const from       = previous
  // A drawn example is a different document rather than an edit to this one,
  // so it publishes whatever its churn reads.
  const drew       = drawn.value !== seenDraw
  seenDraw         = drawn.value
  const html       = await highlight(next, 'python', lintDecorations(diagnostics.value, next))
  if (superseded()) return
  // Mid-edit the display sits behind the editor, so stage the html silently.
  if (editing.value) {
    morphing.value = false
    commit(html, next)
    magicMoveDelta.noteMorphDecision(NO_PAIR, false)
    return
  }
  // Same code with a changed finding set is a lint toggle, so retract the
  // dropped rule's underlines and draw any freshly enabled ones in place,
  // leaving the surviving underlines adhered rather than re-drawing them.
  if (from !== '' && from === next && !reducedMotion.value) {
    morphing.value = false
    await reflow(html, next, superseded)
    magicMoveDelta.noteMorphDecision(NO_PAIR, false)
    return
  }
  if (from === '' || reducedMotion.value) {
    morphing.value = false
    commit(html, next)
    drawSquiggles()
    magicMoveDelta.noteMorphDecision(NO_PAIR, false)
    if (from === '' && !reducedMotion.value) warmPanel(next, superseded)
    return
  }
  // The morph renders a trailing newline as an extra `<br>` line the static
  // display does not carry, so the states trim to the real last line.
  const trimFrom = from.trimEnd()
  const trimNext = next.trimEnd()
  const measure  = magicMoveDelta.lineChurn(trimFrom, trimNext)
  // Past the cap the renderer has real work for most of the panel, which costs
  // more frame time than a reader can follow, so the settled output publishes
  // the way the reduced-motion path publishes it.
  if (drew || measure.churn > magicMoveDelta.MORPH_LINE_CHURN_CAP) {
    morphing.value = false
    commit(html, next)
    drawSquiggles()
    magicMoveDelta.noteMorphDecision(measure, false)
    await advancePanel(trimNext, superseded)
    return
  }
  const pair = await precompile(trimFrom, trimNext)
  if (superseded()) return
  restHeight.value = Math.ceil(surface.value?.getBoundingClientRect().height ?? 0)
  animate.value    = false
  steps.value      = pair.steps
  step.value       = 0
  morphing.value   = true
  // The FLIP morph measures the rest state against the painted DOM, so the
  // revealed panel needs a real frame to lay the "from" tokens out before
  // the step flip. A microtask alone leaves both measurements in one frame
  // and the morph degrades to a jump cut.
  await nextTick()
  await nextPaint()
  if (superseded()) return
  commit(html, next)
  pendingEnds  += 1
  animate.value = true
  step.value    = 1
  magicMoveDelta.noteMorphDecision(pair, true)
  watchdog.start()
}

// Rebuilds the hidden panel at a published state, so the next morph starts
// from what the reader sees rather than from a state two toggles old. The
// leading paint keeps the rebuild off the publish frame, and priming the
// machine from the empty state skips the cross-document token sync no morph
// will ever show.
async function advancePanel(rest: string, superseded: () => boolean): Promise<void> {
  await nextPaint()
  if (superseded()) return
  const pair = await precompile('', rest)
  if (superseded()) return
  animate.value = false
  steps.value   = pair.steps
  step.value    = 1
}

// Mounts the panel behind the settled display and renders one step into it,
// so the first toggle finds the chunks fetched, the token DOM built, and the
// renderer's first-render flag spent. That flag gates the enter and container
// animations, and only an animating render clears it, so the warm-up renders
// with `animate` on. Nothing moves, because the same flag suppresses the
// enters and the panel is hidden. The step-zero gate drops the `end` it emits.
async function warmPanel(text: string, superseded: () => boolean): Promise<void> {
  const rest = text.trimEnd()
  if (rest === '') return
  await nextPaint()
  if (superseded() || steps.value.length > 0) return
  const { steps: warmed } = await precompile(rest, rest)
  if (superseded() || steps.value.length > 0) return
  animate.value = true
  steps.value   = warmed
  step.value    = 0
  await nextTick()
  animate.value = false
}

// The morph settles onto the static display, so the squiggles draw back in
// the way the fixture cards do rather than snapping to full length.
function settleMorph(): void {
  morphing.value = false
  drawSquiggles()
}

// A superseded morph's `end` reaches the live panel, either when the
// successor's swap cancels its transitions or when the successor's flip
// force-resolves them, so the count absorbs it and the step gate keeps it
// from settling a successor still sitting in its rest state.
function endMorph(): void {
  pendingEnds = Math.max(0, pendingEnds - 1)
  if (pendingEnds === 0 && morphing.value && step.value === 1) settleMorph()
}

// Publishes the highlighted output and records the text and the rules it
// carries, so a later diagnostics-only change can tell which underlines were
// dropped and a later morph starts from the text the display actually shows.
function commit(html: string, text: string): void {
  displayHtml.value = html
  previous          = text
  shownRules        = ruleCodes.value
}

// A lint toggle keeps the code but changes the finding set, so retract the
// dropped rule's underlines against the current DOM, swap in the new set once
// they clear, then draw any freshly enabled underlines back in.
async function reflow(html: string, text: string, superseded: () => boolean): Promise<void> {
  const removed = shownRules.difference(ruleCodes.value)
  const added   = ruleCodes.value.difference(shownRules)
  if (removed.size > 0 && display.value) {
    markUndrawn(display.value, removed)
    await promiseTimeout(ruleDrawMs())
    if (superseded()) return
  }
  commit(html, text)
  if (added.size === 0) return
  await nextTick()
  if (superseded() || !display.value) return
  const drawing = markUndrawn(display.value, added)
  await nextPaint()
  if (superseded()) return
  drawing.forEach(flag => flag.classList.remove('lint-undrawn'))
}

// Stages the underlines of `rules` scaled to zero and returns the elements,
// so the caller can retract them or lift the class to draw them back.
function markUndrawn(root: HTMLElement, rules: ReadonlySet<string>): HTMLElement[] {
  const matched = [...root.querySelectorAll<HTMLElement>('.lint-flag')]
    .filter(flag => rules.has(flag.dataset.rule ?? ''))
  matched.forEach(flag => flag.classList.add('lint-undrawn'))
  return matched
}

// The box holds the source the reader hands Prose, where the panel behind it
// shows what Prose returned. Seeding it with the formatted output instead
// would feed an already-formatted source back in, and since the formatter is
// idempotent every rewriting rule would then have nothing left to do and drop
// off the panel.
function startEditing(): void {
  popper.value?.hide()
  draft.value    = source.value
  editing.value  = true
  morphing.value = false
  nextTick(() => editor.value?.focus())
}

// Blur leaves the box open, so a reformat only ever runs on the reader asking
// for one and never as a side effect of clicking away.
function applyEdit(): void {
  if (draft.value !== source.value) source.value = draft.value
  editing.value = false
}

function cancelEdit(): void {
  editing.value = false
}

// Disabling a lint rule leaves the formatted text unchanged and only drops
// its diagnostics, so the squiggle layer tracks both. A same-text render
// re-highlights with the new decorations and skips the morph.
watch([formatted, diagnostics], () => render(formatted.value))
onMounted(() => { if (formatted.value) render(formatted.value) })
</script>

<template>
  <section
    ref="surface"
    class="code-panel sandbox-surface panel panel-clip"
    aria-label="Formatted Python"
    :style="{ minHeight: heldHeight }"
  >
    <span
      v-if="guide != null"
      class="sandbox-surface-guide"
      aria-hidden="true"
      :style="{ '--guide-col': guide, '--guide-hue': guideHue || undefined }"
    />
    <header v-show="!editing" class="code-panel-label">app.py</header>
    <SandboxCodeEditor
      v-show="editing"
      ref="editor"
      v-model="draft"
      lang="python"
      @keydown.esc="cancelEdit"
      @keydown.enter.meta="applyEdit"
      @keydown.enter.ctrl="applyEdit"
    />
    <div v-show="editing" class="sandbox-surface-actions">
      <button
        type="button"
        class="panel-seat sandbox-surface-action sandbox-surface-discard"
        title="Discard the edit"
        aria-label="Discard the edit"
        @click="cancelEdit"
      >
        <svg class="glyph" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path d="M6 6l12 12M18 6L6 18" />
        </svg>
      </button>
      <button
        type="button"
        class="panel-seat sandbox-surface-action sandbox-surface-apply"
        title="Format this source"
        aria-label="Format this source"
        @click="applyEdit"
      >
        <svg class="glyph" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path d="M4 12.5l5 5L20 6.5" />
        </svg>
      </button>
    </div>
    <component
      :is="panel"
      v-if="panel && morphSteps.length"
      v-show="morphing && !editing"
      class="code-panel-code"
      :steps="morphSteps"
      :step="step"
      :animate="animate && !reducedMotion"
      :options="morphOptions"
      @end="endMorph"
    />
    <div
      v-show="!morphing && !editing"
      ref="display"
      class="code-panel-code code-panel-editable sandbox-surface-display"
      :class="{ 'lint-undrawn': undrawn }"
      role="button"
      tabindex="0"
      @click="startEditing"
      @keydown.enter.prevent="startEditing"
      @mouseover="popper?.show"
      @mouseout="popper?.hide"
      @focusin="popper?.show"
      @focusout="popper?.hide"
      v-html="displayHtml"
    />
    <p v-if="error" class="code-panel-error">{{ error }}</p>
    <p v-if="unstable.length" class="code-panel-unstable">
      A second run would change this output ({{ unstable.join(', ') }}), which is a defect in
      Prose itself.
      <a :href="reportUrl" v-bind="externalAttrs(reportUrl)">Report it</a>
      with the source above.
    </p>
    <LintFlagPopper ref="popper" />
  </section>
</template>

<style scoped>
/* Mid-resize the new layout overflows the still-animating height, so the
   morph pane clips instead of flashing a scrollbar. */
.sandbox-surface :deep(.shiki-magic-move-container) {
  overflow : hidden;
}

.sandbox-surface-actions {
  position : absolute;
  right    : 10px;
  bottom   : 8px;
  z-index  : 4;
  display  : flex;
  gap      : 4px;
}

/* The corners reveal on hover, whereas these hold visible, the pane having no
   other way out. */
.sandbox-surface-action {
  transition : color var(--prose-transition), border-color var(--prose-transition);
}

.sandbox-surface-apply:hover {
  border-color : var(--vp-c-brand-1);
  color        : var(--vp-c-brand-1);
}

.sandbox-surface-guide {
  position       : absolute;
  top            : 1px;
  bottom         : 1px;
  left           : calc(22px + var(--guide-col) * 1ch);
  width          : 0;
  border-left    : 2px dotted color-mix(in srgb, var(--guide-hue, var(--prose-palette-ube)) 75%, transparent);
  font-family    : var(--vp-font-family-mono);
  font-size      : var(--prose-text-xs);
  pointer-events : none;
}
</style>
