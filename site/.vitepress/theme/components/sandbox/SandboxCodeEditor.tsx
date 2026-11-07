import { computedAsync }                         from '@vueuse/core'
import { defineComponent, shallowRef, useModel } from 'vue'
import type { PropType }                         from 'vue'

import { highlight } from '../../../lib/shared/highlight'

export interface SandboxCodeEditorHandle {
  focus : () => void
}

export const SandboxCodeEditor = defineComponent({
  name  : 'SandboxCodeEditor',
  props : {
    lang       : { required: true, type: String as PropType<'python' | 'toml'> },
    modelValue : { required: true, type: String }
  },
  emits : { blur: () => true, keydown: (event: KeyboardEvent) => true, 'update:modelValue': (value: string) => true },
  setup(props, { emit, expose }) {
    const model = useModel(props, 'modelValue')

    const layer = shallowRef<HTMLElement | null>(null)
    const input = shallowRef<HTMLTextAreaElement | null>(null)

    // A trailing newline collapses in the highlight layer but not the
    // textarea, so pad it with a space to keep the two boxes the same height.
    const highlighted = computedAsync(() => {
      const text = model.value
      return highlight(text.endsWith('\n') ? `${text} ` : text, props.lang)
    }, '', { flush: 'pre' })

    function syncScroll(): void {
      if (layer.value && input.value) layer.value.scrollLeft = input.value.scrollLeft
    }

    expose({ focus: () => input.value?.focus() })

    return () => (
      <div class="sandbox-code-editor relative flex grow flex-col">
        <div
          aria-hidden="true"
          class="sandbox-code-editor-layer code-panel-code whitespace-pre"
          innerHTML={highlighted.value}
          ref={layer}
        />
        <textarea
          autocapitalize="off"
          autocomplete="off"
          autocorrect="off"
          class="sandbox-code-editor-layer absolute inset-0 code-panel-code resize-none rounded-base-inner border-none whitespace-pre text-transparent caret-text-1 focus-visible:-outline-offset-2"
          onBlur={() => emit('blur')}
          onKeydown={event => emit('keydown', event)}
          onScroll={syncScroll}
          ref={input}
          spellcheck="false"
          v-model={model.value}
        />
      </div>
    )
  }
})
