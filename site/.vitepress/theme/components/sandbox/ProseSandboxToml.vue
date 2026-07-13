<script setup lang="ts">
import { promiseTimeout, useClipboard }                    from '@vueuse/core'
import { nextTick, onMounted, ref, useTemplateRef, watch } from 'vue'

import SandboxCodeEditor from './SandboxCodeEditor.vue'

import type { ProseSandbox } from '../../../lib/composables/use-prose-sandbox'
import { useReducedMotion }  from '../../../lib/composables/use-reduced-motion'
import { highlight }         from '../../../lib/sandbox/highlight'
import * as typewriter       from '../../../lib/sandbox/typewriter'

const CARET   = '<span class="code-caret" aria-hidden="true"></span>'
const STEP_MS = 12

const props = defineProps<{ sandbox: ProseSandbox }>()
const { configError, configToml } = props.sandbox

const { copy, copied } = useClipboard({ source: configToml })

const reducedMotion = useReducedMotion()
const editor        = useTemplateRef<InstanceType<typeof SandboxCodeEditor>>('editor')

const displayHtml = ref('')
const editing     = ref(false)
const shown       = ref('')
const typing      = ref(false)
const typingHtml  = ref('')

let generation = 0

async function settle(text: string): Promise<void> {
  displayHtml.value = text.trim() ? await highlight(text, 'toml') : ''
  typing.value      = false
}

// Types the panel-driven config change in the settled colors, tokenizing the
// before and after once and rendering every frame from those tokens. A
// single-line change sweeps as one caret from the shared character, whereas
// a bulk change backspaces and retypes every affected line concurrently,
// each under its own caret, the untouched lines holding still. A newer
// change bumps the generation and abandons the stale run.
async function typeTo(next: string): Promise<void> {
  const gen = ++generation
  if (reducedMotion.value) {
    shown.value = next
    await settle(next)
    return
  }
  const current = shown.value
  const [curTokens, nextTokens] =
    await Promise.all([typewriter.tokenLines(current), typewriter.tokenLines(next)])
  if (gen !== generation) return
  const plan = typewriter.typingPlan(current, next)

  function frame(
    tokens : typewriter.TokenLine[],
    lines  : readonly string[],
    midEnd : number,
    chars  : number
  ): void {
    const parts: string[] = []
    const texts: string[] = []
    for (let index = 0; index < lines.length; index += 1) {
      if (index < plan.prefix || index >= midEnd) {
        parts.push(typewriter.lineHtml(tokens[index] ?? [], Number.POSITIVE_INFINITY))
        texts.push(lines[index])
        continue
      }
      const visible = Math.min(chars, lines[index].length)
      parts.push(typewriter.lineHtml(tokens[index] ?? [], visible) + CARET)
      texts.push(lines[index].slice(0, visible))
    }
    typingHtml.value = parts.join('\n')
    shown.value      = texts.join('\n')
  }

  typing.value = true
  for (let chars = plan.curMax; chars > plan.floor; chars -= 1) {
    if (gen !== generation) return
    frame(curTokens, plan.curLines, plan.curMidEnd, chars - 1)
    await promiseTimeout(STEP_MS)
  }
  for (let chars = plan.floor; chars < plan.nextMax; chars += 1) {
    if (gen !== generation) return
    frame(nextTokens, plan.nextLines, plan.nextMidEnd, chars + 1)
    await promiseTimeout(STEP_MS)
  }
  if (gen !== generation) return
  shown.value = next
  await settle(next)
}

function startEditing(): void {
  editing.value = true
  nextTick(() => editor.value?.focus())
}

function stopEditing(): void {
  editing.value = false
  shown.value   = configToml.value
  settle(configToml.value)
}

watch(configToml, next => { if (!editing.value) typeTo(next) })

onMounted(() => {
  shown.value = configToml.value
  settle(configToml.value)
})
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
      class="code-panel-code sandbox-toml-display"
      role="button"
      tabindex="0"
      @click="startEditing"
      @keydown.enter.prevent="startEditing"
      v-html="displayHtml"
    />
    <button
      v-show="!editing"
      type="button"
      class="copy"
      :class="{ copied }"
      :title="copied ? 'Copied' : 'Copy prose.toml'"
      @click="copy()"
    />
    <p v-if="configError" class="code-panel-error">{{ configError }}</p>
  </section>
</template>

<style scoped>
.sandbox-toml-display {
  cursor : text;
}
</style>
