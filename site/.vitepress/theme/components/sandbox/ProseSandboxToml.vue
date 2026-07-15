<script setup lang="ts">
import { promiseTimeout }                                  from '@vueuse/core'
import { nextTick, onMounted, ref, useTemplateRef, watch } from 'vue'

import CopyButton        from '../base/CopyButton.vue'
import SandboxCodeEditor from './SandboxCodeEditor.vue'

import type { ProseSandbox } from '../../../lib/composables/use-prose-sandbox'
import { useReducedMotion }  from '../../../lib/composables/use-reduced-motion'
import { highlight }         from '../../../lib/sandbox/highlight'
import * as typewriter       from '../../../lib/sandbox/typewriter'
import { latestRun }         from '../../../lib/shared/latest-run'

const STEP_MS = 12

const props = defineProps<{ sandbox: ProseSandbox }>()
const { configError, configToml } = props.sandbox

const reducedMotion = useReducedMotion()
const editor        = useTemplateRef<InstanceType<typeof SandboxCodeEditor>>('editor')

const displayHtml = ref('')
const editing     = ref(false)
const typing      = ref(false)
const typingHtml  = ref('')

const run = latestRun()

let shown = ''

async function settle(text: string): Promise<void> {
  const superseded = run.begin()
  shown = text
  const html = text.trim() ? await highlight(text, 'toml') : ''
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

onMounted(() => settle(configToml.value))
</script>

<template>
  <section class="code-panel sandbox-toml copy-host panel panel-clip" aria-label="prose.toml config">
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
      class="code-panel-code code-typewriter shiki"
      v-html="typingHtml"
    />
    <div
      v-show="!editing && !typing"
      class="code-panel-code code-panel-editable sandbox-toml-display"
      role="button"
      tabindex="0"
      @click="startEditing"
      @keydown.enter.prevent="startEditing"
      v-html="displayHtml"
    />
    <CopyButton v-show="!editing" label="Copy prose.toml" :source="configToml" />
    <p v-if="configError" class="code-panel-error">{{ configError }}</p>
  </section>
</template>
