<script setup lang="ts">
import { computedAsync }  from '@vueuse/core'
import { useTemplateRef } from 'vue'

import { highlight } from '../../../lib/shared/highlight'

const props = defineProps<{ lang: 'python' | 'toml' }>()
const model = defineModel<string>({ required: true })

defineEmits<{ blur: [] }>()

const layer = useTemplateRef<HTMLElement>('layer')
const input = useTemplateRef<HTMLTextAreaElement>('input')

// A trailing newline collapses in the highlight layer but not the
// textarea, so pad it with a space to keep the two boxes the same height.
const highlighted = computedAsync(() => {
  const text = model.value
  return highlight(text.endsWith('\n') ? `${text} ` : text, props.lang)
}, '', { flush: 'pre' })

function syncScroll(): void {
  if (layer.value && input.value) layer.value.scrollLeft = input.value.scrollLeft
}

defineExpose({ focus: () => input.value?.focus() })
</script>

<template>
  <div class="sandbox-code-editor">
    <div ref="layer" class="code-panel-code sandbox-code-editor-layer" aria-hidden="true" v-html="highlighted" />
    <textarea
      ref="input"
      v-model="model"
      class="code-panel-code sandbox-code-editor-layer sandbox-code-editor-input"
      autocapitalize="off"
      autocomplete="off"
      autocorrect="off"
      spellcheck="false"
      @blur="$emit('blur')"
      @scroll="syncScroll"
    />
  </div>
</template>
