<script setup lang="ts">
import type { KeyedTokensInfo } from '@shikijs/magic-move/types'
import { promiseTimeout }       from '@vueuse/core'
import { nextTick, onMounted, ref, shallowRef, useTemplateRef, watch } from 'vue'

import LintFlagPopper    from '../rules/LintFlagPopper.vue'
import SandboxCodeEditor from './SandboxCodeEditor.vue'

import type { ProseSandbox }     from '../../../lib/composables/use-prose-sandbox'
import { useReducedMotion }      from '../../../lib/composables/use-reduced-motion'
import { lintDecorations }       from '../../../lib/markdown/lint-decorations'
import { magicMoveOptions, type MagicMovePanel } from '../../../lib/markdown/magic-move-options'
import { highlight }             from '../../../lib/sandbox/highlight'
import { nextPaint, ruleDrawMs } from '../../../lib/shared/paint'

const props = defineProps<{ guide?: number | null, guideHue?: string, sandbox: ProseSandbox }>()
const { diagnostics, error, formatted, source } = props.sandbox

const MORPH_MS = 450

const reducedMotion = useReducedMotion()
const display       = useTemplateRef<HTMLElement>('display')
const editor        = useTemplateRef<InstanceType<typeof SandboxCodeEditor>>('editor')
const popper        = useTemplateRef<InstanceType<typeof LintFlagPopper>>('popper')

const animate     = ref(false)
const displayHtml = ref('')
const draft       = ref('')
const editing     = ref(false)
const morphKey    = ref(0)
const morphing    = ref(false)
const panel       = shallowRef<MagicMovePanel>(null)
const step        = ref(0)
const steps       = shallowRef<readonly KeyedTokensInfo[]>([])
const undrawn     = ref(false)

let previous   = ''
let generation = 0
let shownRules = new Set<string>()

// Renders the settled output as highlighted HTML carrying the lint
// squiggles, then morphs from the prior output when motion is allowed and
// the surface is not mid-edit. Each transition mounts a fresh magic-move
// instance keyed by `morphKey`, so it measures its rest state before the
// step flip animates it, matching the fixture morph. A newer change bumps
// the generation, so a superseded render abandons rather than racing on the
// shared morph state, and a watchdog restores the static display if the
// `@end` event is ever missed.
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
  commit(html)
  if (from === '' || from === next || reducedMotion.value) {
    drawSquiggles()
    return
  }
  if (!panel.value) {
    panel.value = (await import('@shikijs/magic-move/vue')).ShikiMagicMovePrecompiled
  }
  const { precompileMagicMove } = await import('../../../lib/markdown/magic-move')
  const committed = await precompileMagicMove([from, next])
  if (gen !== generation) return
  steps.value     = committed
  step.value      = 0
  animate.value   = false
  morphKey.value += 1
  morphing.value  = true
  // The FLIP morph measures the rest state against the painted DOM, so the
  // fresh instance needs a real frame to lay the "from" tokens out before
  // the step flip. A microtask alone leaves both measurements in one frame
  // and the morph degrades to a jump cut.
  await nextTick()
  await nextPaint()
  if (gen !== generation) return
  animate.value = true
  step.value    = 1
  setTimeout(() => { if (gen === generation && morphing.value) endMorph() }, MORPH_MS + 250)
}

// The morph settles onto the static display, so the squiggles draw back in
// the way the fixture cards do rather than snapping to full length.
function endMorph(): void {
  morphing.value = false
  drawSquiggles()
}

// Publishes the highlighted output and records which rules it carries, so a
// later diagnostics-only change can tell which underlines were dropped.
function commit(html: string): void {
  displayHtml.value = html
  shownRules        = new Set(diagnostics.value.map(finding => finding.code))
}

// A lint toggle keeps the code but changes the finding set: retract the
// dropped rule's underlines against the current DOM, swap in the new set once
// they clear, then draw any freshly enabled underlines back in.
async function reflow(html: string, gen: number): Promise<void> {
  const nextRules = new Set(diagnostics.value.map(finding => finding.code))
  const removed   = [...shownRules].filter(rule => !nextRules.has(rule))
  const added     = [...nextRules].filter(rule => !shownRules.has(rule))
  if (removed.length > 0 && display.value) {
    markUndrawn(display.value, removed)
    await promiseTimeout(ruleDrawMs())
    if (gen !== generation) return
  }
  commit(html)
  if (added.length === 0) return
  await nextTick()
  if (gen !== generation || !display.value) return
  const drawing = markUndrawn(display.value, added)
  await nextPaint()
  drawing.forEach(flag => flag.classList.remove('lint-undrawn'))
}

// Stages the underlines of `rules` scaled to zero and returns the elements,
// so the caller can retract them or lift the class to draw them back.
function markUndrawn(root: HTMLElement, rules: readonly string[]): HTMLElement[] {
  const matched: HTMLElement[] = []
  root.querySelectorAll<HTMLElement>('.lint-flag').forEach(flag => {
    if (rules.includes(flag.dataset.rule ?? '')) {
      flag.classList.add('lint-undrawn')
      matched.push(flag)
    }
  })
  return matched
}

// Stages the lint underlines undrawn, then lifts the class after a paint
// so their scaleX transition re-fires left to right.
async function drawSquiggles(): Promise<void> {
  if (typeof requestAnimationFrame === 'undefined') return
  undrawn.value = true
  await nextPaint()
  undrawn.value = false
}

// The flat character offset under a point, walking the display's text nodes
// up to the caret node the browser resolves there.
function offsetAt(root: HTMLElement, x: number, y: number): number {
  const position = document.caretPositionFromPoint?.(x, y)
  const range    = position ? undefined : document.caretRangeFromPoint?.(x, y)
  const target   = position?.offsetNode ?? range?.startContainer
  const inNode   = position?.offset ?? range?.startOffset ?? 0
  if (!target) return 0
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT)
  let total = 0
  for (let text = walker.nextNode(); text; text = walker.nextNode()) {
    if (text === target) return total + inNode
    total += text.textContent?.length ?? 0
  }
  return total
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
  nextTick(() => {
    editor.value?.focus()
    if (offset !== undefined) editor.value?.caret(offset)
  })
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
      :steps="[...steps]"
      :step="step"
      :animate="animate && !reducedMotion"
      :options="magicMoveOptions(MORPH_MS)"
      @end="endMorph"
    />
    <div
      v-show="!morphing && !editing"
      ref="display"
      class="code-panel-code sandbox-surface-display"
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
.sandbox-surface {
  min-height : 30rem;
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

.sandbox-surface-display {
  cursor : text;
}
</style>
