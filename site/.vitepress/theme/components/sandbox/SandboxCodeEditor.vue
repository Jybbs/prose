<script setup lang="ts">
import { ref, useTemplateRef, watch } from 'vue'

import { highlight } from '../../../lib/sandbox/highlight'

const props = defineProps<{ lang: 'python' | 'toml' }>()
const model = defineModel<string>({ required: true })
const emit  = defineEmits<{ blur: [] }>()

const highlighted = ref('')
const layer       = useTemplateRef<HTMLElement>('layer')
const input       = useTemplateRef<HTMLTextAreaElement>('input')

// A trailing newline collapses in the highlight layer but not the
// textarea, so pad it with a space to keep the two boxes the same height.
async function render(text: string): Promise<void> {
  highlighted.value = await highlight(text.endsWith('\n') ? `${text} ` : text, props.lang)
}

function syncScroll(): void {
  if (!layer.value || !input.value) return
  layer.value.scrollTop  = input.value.scrollTop
  layer.value.scrollLeft = input.value.scrollLeft
}

watch(model, render, { immediate: true })

defineExpose({
  caret : (offset: number) => input.value?.setSelectionRange(offset, offset),
  focus : () => input.value?.focus()
})
</script>

<template>
  <div class="code-editor">
    <div ref="layer" class="code-panel-code code-editor-layer" aria-hidden="true" v-html="highlighted" />
    <textarea
      ref="input"
      v-model="model"
      class="code-panel-code code-editor-layer code-editor-input"
      autocapitalize="off"
      autocomplete="off"
      autocorrect="off"
      spellcheck="false"
      @blur="emit('blur')"
      @scroll="syncScroll"
    />
  </div>
</template>

<style scoped>
.code-editor {
  position  : relative;
  flex-grow : 1;
}

.code-editor-layer {
  position    : absolute;
  inset       : 0;
  white-space : pre;
  tab-size    : 4;
}

.code-editor-input {
  border        : 0;
  border-radius : calc(var(--prose-radius) - 1px);
  background    : transparent;
  color         : transparent;
  caret-color   : var(--vp-c-text-1);
  resize        : none;
  outline       : none;
}

.code-editor-input:focus-visible {
  outline        : var(--prose-focus-ring);
  outline-offset : -2px;
}
</style>
