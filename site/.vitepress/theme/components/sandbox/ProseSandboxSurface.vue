<script setup lang="ts">
import type { KeyedTokensInfo }         from '@shikijs/magic-move/types'
import { promiseTimeout, useTimeoutFn } from '@vueuse/core'
import { computed, nextTick, onMounted, ref, shallowRef, useTemplateRef, watch } from 'vue'

import LintFlagPopper    from '../rules/LintFlagPopper.vue'
import SandboxCodeEditor from './SandboxCodeEditor.vue'

import type { ProseSandbox }     from '../../../lib/composables/use-prose-sandbox'
import { useReducedMotion }      from '../../../lib/composables/use-reduced-motion'
import { useSquiggleDraw }       from '../../../lib/composables/use-squiggle-draw'
import { lintDecorations }       from '../../../lib/markdown/lint-decorations'
import * as magicMove            from '../../../lib/markdown/magic-move-options'
import { offsetAt }              from '../../../lib/sandbox/caret'
import { highlight }             from '../../../lib/sandbox/highlight'
import { nextPaint, ruleDrawMs } from '../../../lib/shared/paint'

const props = defineProps<{ guide?: number | null, guideHue?: string, sandbox: ProseSandbox }>()
const { diagnostics, error, formatted, source } = props.sandbox

const reducedMotion = useReducedMotion()
const display       = useTemplateRef<HTMLElement>('display')
const editor        = useTemplateRef<InstanceType<typeof SandboxCodeEditor>>('editor')
const popper        = useTemplateRef<InstanceType<typeof LintFlagPopper>>('popper')

const displayHtml = ref('')
const draft       = ref('')
const editing     = ref(false)
const morphKey    = ref(0)
const morphMs     = ref(450)
const morphing    = ref(false)
const panel       = shallowRef<magicMove.MagicMovePanel>(null)
const step        = ref(0)
const steps       = shallowRef<readonly KeyedTokensInfo[]>([])

const { drawSquiggles, undrawn } = useSquiggleDraw()

// The precompiled panel re-syncs its keys, with in-place side effects,
// whenever these prop identities change, so they stay stable across
// unrelated re-renders instead of rebuilding per template pass.
const morphOptions = computed(() => magicMove.magicMoveOptions(morphMs.value, 0))
const morphSteps   = computed(() => [...steps.value])

const ruleCodes = computed(() => new Set(diagnostics.value.map(finding => finding.code)))

const watchdog = useTimeoutFn(
  () => { if (morphing.value) endMorph() },
  () => morphMs.value + 250,
  { immediate: false }
)

let previous   = ''
let generation = 0
let shownRules = new Set<string>()

// Renders the settled output as highlighted HTML carrying the lint
// squiggles, then morphs from the prior output when motion is allowed and
// the surface is not mid-edit. Each transition mounts a fresh magic-move
// instance keyed by `morphKey`, so it measures its rest state before the
// step flip animates it, matching the fixture morph. The settled html
// lands on the static display only under the morph's cover, so the pane
// never flashes the end state before the tokens slide. A newer change
// bumps the generation, so a superseded render abandons rather than
// racing on the shared morph state, and a watchdog restores the static
// display if the `@end` event is ever missed.
async function render(next: string): Promise<void> {
  const gen  = ++generation
  const from = previous
  const html = await highlight(next, 'python', lintDecorations(diagnostics.value))
  if (gen !== generation) return
  previous = next
  // Mid-edit the display sits behind the editor, so stage the html silently.
  if (editing.value) { commit(html); return }
  // Same code with a changed finding set is a lint toggle, so retract the
  // dropped rule's underlines and draw any freshly enabled ones in place,
  // leaving the surviving underlines adhered rather than re-drawing them.
  if (from !== '' && from === next && !reducedMotion.value) {
    await reflow(html, gen)
    return
  }
  if (from === '' || reducedMotion.value) {
    commit(html)
    drawSquiggles()
    return
  }
  panel.value ??= (await import('@shikijs/magic-move/vue')).ShikiMagicMovePrecompiled
  const { precompileMagicMove } = await import('../../../lib/markdown/magic-move')
  // The morph renders a trailing newline as an extra `<br>` line the static
  // display does not carry, so the states trim to the real last line.
  const committed = await precompileMagicMove([from.trimEnd(), next.trimEnd()])
  if (gen !== generation) return
  steps.value     = committed
  step.value      = 0
  morphMs.value   = ruleDrawMs()
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
  if (gen !== generation) return
  commit(html)
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

// Publishes the highlighted output and records which rules it carries, so a
// later diagnostics-only change can tell which underlines were dropped.
function commit(html: string): void {
  displayHtml.value = html
  shownRules        = ruleCodes.value
}

// A lint toggle keeps the code but changes the finding set, so retract the
// dropped rule's underlines against the current DOM, swap in the new set once
// they clear, then draw any freshly enabled underlines back in.
async function reflow(html: string, gen: number): Promise<void> {
  const removed = shownRules.difference(ruleCodes.value)
  const added   = ruleCodes.value.difference(shownRules)
  if (removed.size > 0 && display.value) {
    markUndrawn(display.value, removed)
    await promiseTimeout(ruleDrawMs())
    if (gen !== generation) return
  }
  commit(html)
  if (added.size === 0) return
  await nextTick()
  if (gen !== generation || !display.value) return
  const drawing = markUndrawn(display.value, added)
  await nextPaint()
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

// Editing works on the resultant text, so the box opens seeded with the
// formatted output rather than the pristine source, with the caret landing
// where the click did. A genuine edit is adopted as the new source and
// reformats, whereas a no-op click leaves the original source in place so
// a later rule toggle still reformats from it.
function startEditing(event: MouseEvent | KeyboardEvent): void {
  const offset = event instanceof MouseEvent && display.value
    ? offsetAt(display.value, event.clientX, event.clientY)
    : undefined
  popper.value?.hide()
  draft.value    = formatted.value
  editing.value  = true
  morphing.value = false
  nextTick(() => editor.value?.focus(offset))
}

function stopEditing(): void {
  if (draft.value !== formatted.value) source.value = draft.value
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
    <header class="code-panel-label">app.py</header>
    <SandboxCodeEditor
      v-show="editing"
      ref="editor"
      v-model="draft"
      lang="python"
      @blur="stopEditing"
    />
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
