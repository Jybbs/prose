<script setup lang="ts">
import { promiseTimeout }                                            from '@vueuse/core'
import { computed, nextTick, onMounted, ref, useTemplateRef, watch } from 'vue'

import ConfigFlagPopper  from './ConfigFlagPopper.vue'
import CopyButton        from '../base/CopyButton.vue'
import SandboxCodeEditor from './SandboxCodeEditor.vue'

import type { ProseSandbox }                    from '../../../lib/composables/use-prose-sandbox'
import { useReducedMotion }                     from '../../../lib/composables/use-reduced-motion'
import { configDecorations, unknownKeyFindings } from '../../../lib/sandbox/config-decorations'
import * as typewriter                          from '../../../lib/sandbox/typewriter'
import { highlight }                            from '../../../lib/shared/highlight'
import { latestRun }                            from '../../../lib/shared/latest-run'

const STEP_MS = 12

const props = defineProps<{ sandbox: ProseSandbox }>()
const { configError, configNotices, configToml, renameConfigKey, rules } = props.sandbox

const reducedMotion = useReducedMotion()
const editor        = useTemplateRef<InstanceType<typeof SandboxCodeEditor>>('editor')
const popper        = useTemplateRef<InstanceType<typeof ConfigFlagPopper>>('popper')

// An unknown key the source carries takes a squiggle and a margin mark,
// leaving the message list to the notices no line matched.
const located = computed(() => unknownKeyFindings(configNotices.value, configToml.value))
const flagged = computed(() => new Set(located.value.map(finding => finding.location.row)))
const rows    = computed(() => configToml.value.split('\n').length)
const stray   = computed(() => {
  const matched = new Set(located.value.map(finding => finding.message))
  return configNotices.value.filter(notice => !matched.has(notice))
})

const displayHtml = ref('')
const editing     = ref(false)
const typing      = ref(false)
const typingHtml  = ref('')

const run = latestRun()

let shown = ''

async function settle(text: string): Promise<void> {
  const superseded = run.begin()
  shown = text
  const html = text.trim()
    ? await highlight(text, 'toml', configDecorations(configNotices.value, text))
    : ''
  if (superseded()) return
  displayHtml.value = html
  typing.value      = false
}

// Types the panel-driven config change in the settled colors, tokenizing the
// before and after once and rendering every frame from those tokens. A
// single-line change sweeps as one caret from the shared character, whereas
// a bulk change backspaces and retypes every affected line concurrently,
// each under its own caret, the untouched lines holding still. A newer
// change supersedes the run in flight, which abandons its sweep.
async function typeTo(next: string): Promise<void> {
  const superseded = run.begin()
  if (reducedMotion.value) {
    await settle(next)
    return
  }
  const current = shown
  const [curTokens, nextTokens] =
    await Promise.all([typewriter.tokenLines(current), typewriter.tokenLines(next)])
  if (superseded()) return
  const plan = typewriter.typingPlan(current, next)

  // Steps the caret from `from` to `to`, backspacing when the target is
  // lower and typing when it is higher, and abandons on a newer change.
  async function sweep(
    tokens : typewriter.TokenLine[],
    side   : typewriter.TypingSide,
    from   : number,
    to     : number
  ): Promise<void> {
    const step = from < to ? 1 : -1
    for (let chars = from; chars !== to; chars += step) {
      if (superseded()) return
      const { html, text } = typewriter.typingFrame(tokens, side, plan.prefix, chars + step)
      typingHtml.value = html
      shown            = text
      await promiseTimeout(STEP_MS)
    }
  }

  typing.value = true
  await sweep(curTokens, plan.cur, plan.cur.max, plan.floor)
  if (superseded()) return
  await sweep(nextTokens, plan.next, plan.floor, plan.next.max)
  if (superseded()) return
  await settle(next)
}

function startEditing(): void {
  run.cancel()
  editing.value = true
  nextTick(() => editor.value?.focus())
}

function stopEditing(): void {
  editing.value = false
  settle(configToml.value)
}

watch(configToml, next => { if (!editing.value) typeTo(next) })

// The notices land a beat after the text they describe, so the panel repaints
// against the current config rather than waiting for the next edit.
watch(configNotices, () => { if (!editing.value) settle(configToml.value) })

onMounted(() => settle(configToml.value))
</script>

<template>
  <section class="sandbox-toml copy-host code-panel overflow-hidden panel transition-opacity in-data-editing:pointer-events-none in-data-editing:opacity-50" aria-label="prose.toml config">
    <header class="code-panel-label">prose.toml</header>

    <SandboxCodeEditor
      v-show="editing"
      ref="editor"
      v-model="configToml"
      lang="toml"
      @blur="stopEditing"
    />
    <pre
      v-show="!editing && typing"
      class="shiki code-panel-code code-typewriter"
      v-html="typingHtml"
    />
    <div v-show="!editing && !typing" class="flex min-h-0">
      <div v-if="located.length" class="sandbox-toml-gutter shrink-0 py-18 pl-10 text-xs leading-1-6 select-none" aria-hidden="true">
        <span v-for="row in rows" :key="row" :data-flagged="flagged.has(row) ? '' : undefined" />
      </div>
      <div
        class="sandbox-toml-display code-text grow cursor-text overflow-auto py-18 pr-22 pl-8"
        role="button"
        tabindex="0"
        @click="startEditing"
        @keydown.enter.prevent="startEditing"
        @mouseover="popper?.show"
        @mouseleave="popper?.hide"
        v-html="displayHtml"
      />
    </div>
    <p v-if="located.length" class="config-notice-strip flex items-center gap-6 border-t-1 border-t-divider bg-warning-1/10 px-14 py-7 font-mono text-xs tracking-0-02em text-warning-1">
      <span aria-hidden="true">▲</span>
      {{ located.length }} unknown {{ located.length === 1 ? 'key' : 'keys' }}
    </p>
    <CopyButton v-show="!editing" label="Copy prose.toml" :source="configToml" />
    <p v-if="configError" class="code-panel-error code-panel-notice text-danger-1">{{ configError }}</p>
    <p v-for="notice in stray" :key="notice" class="code-panel-warning code-panel-notice text-warning-1">{{ notice }}</p>
    <ConfigFlagPopper ref="popper" :rules="rules" @apply="renameConfigKey" />
  </section>
</template>
