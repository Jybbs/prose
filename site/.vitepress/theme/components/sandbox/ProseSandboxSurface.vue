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
import { highlight }             from '../../../lib/sandbox/highlight'
import { latestRun }             from '../../../lib/shared/latest-run'
import { nextPaint, ruleDrawMs } from '../../../lib/shared/paint'

const props   = defineProps<{ guide?: number | null, guideHue?: string, sandbox: ProseSandbox }>()
const editing = defineModel<boolean>('editing', { default: false })
const { diagnostics, error, formatted, source } = props.sandbox

const reducedMotion = useReducedMotion()
const display       = useTemplateRef<HTMLElement>('display')
const editor        = useTemplateRef<InstanceType<typeof SandboxCodeEditor>>('editor')
const popper        = useTemplateRef<InstanceType<typeof LintFlagPopper>>('popper')

const displayHtml = ref('')
const draft       = ref('')
const morphKey    = ref(0)
const morphing    = ref(false)
const step        = ref(0)

const { drawSquiggles, undrawn } = useSquiggleDraw()

const { duration, morphOptions, morphSteps, panel, precompile, steps } = useMagicMove(0)

const ruleCodes = computed(() => new Set(diagnostics.value.map(finding => finding.code)))

const watchdog = useTimeoutFn(
  () => { if (morphing.value) endMorph() },
  () => duration.value + 250,
  { immediate: false }
)

const run = latestRun()

let previous   = ''
let shownRules = new Set<string>()

// Renders the settled output as highlighted HTML carrying the lint
// squiggles, then morphs from the prior output when motion is allowed and
// the surface is not mid-edit. Each transition mounts a fresh magic-move
// instance keyed by `morphKey`, so it measures its rest state before the
// step flip animates it, matching the fixture morph. The settled html
// lands on the static display only under the morph's cover, so the pane
// never flashes the end state before the tokens slide. A newer change
// supersedes the render in flight, so it abandons rather than racing on
// the shared morph state, and a watchdog restores the static display if
// the `@end` event is ever missed.
async function render(next: string): Promise<void> {
  const superseded = run.begin()
  const from       = previous
  const html       = await highlight(next, 'python', lintDecorations(diagnostics.value))
  if (superseded()) return
  // Mid-edit the display sits behind the editor, so stage the html silently.
  if (editing.value) { commit(html, next); return }
  // Same code with a changed finding set is a lint toggle, so retract the
  // dropped rule's underlines and draw any freshly enabled ones in place,
  // leaving the surviving underlines adhered rather than re-drawing them.
  if (from !== '' && from === next && !reducedMotion.value) {
    await reflow(html, next, superseded)
    return
  }
  if (from === '' || reducedMotion.value) {
    commit(html, next)
    drawSquiggles()
    return
  }
  // The morph renders a trailing newline as an extra `<br>` line the static
  // display does not carry, so the states trim to the real last line.
  const committed = await precompile(from.trimEnd(), next.trimEnd())
  if (superseded()) return
  steps.value     = committed
  step.value      = 0
  morphKey.value += 1
  morphing.value  = true
  // The FLIP morph measures the rest state against the painted DOM, so the
  // fresh instance needs a real frame to lay the "from" tokens out before
  // the step flip. A microtask alone leaves both measurements in one frame
  // and the morph degrades to a jump cut. Mounting with `animate` on runs
  // the rest state through a real render, so the flip is no longer the
  // renderer's first and its container-height transition engages.
  await nextTick()
  await nextPaint()
  if (superseded()) return
  commit(html, next)
  step.value = 1
  watchdog.start()
}

// The morph settles onto the static display, so the squiggles draw back in
// the way the fixture cards do rather than snapping to full length. The
// mount's rest-state render emits its own `end` before the flip, which the
// step guard drops.
function endMorph(): void {
  if (step.value === 0) return
  morphing.value = false
  drawSquiggles()
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
  <section class="code-panel sandbox-surface panel panel-clip" aria-label="Formatted Python">
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
      v-if="panel && morphing"
      :key="morphKey"
      v-show="!editing"
      class="code-panel-code"
      :steps="morphSteps"
      :step="step"
      :animate="!reducedMotion"
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
