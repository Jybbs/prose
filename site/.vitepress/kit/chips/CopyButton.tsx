import { useClipboard }    from '@vueuse/core'
import { defineComponent } from 'vue'

// Copies `source` to the clipboard, its glyph and its label reporting the copy
// for `copiedDuring` milliseconds.
export const CopyButton = defineComponent({
  name  : 'CopyButton',
  props : {
    copiedDuring : Number,
    label        : { required: true, type: String },
    source       : { required: true, type: String }
  },
  setup(props) {
    const { copied, copy } = useClipboard({ copiedDuring: props.copiedDuring })

    return () => {
      const label = copied.value ? 'Copied' : props.label
      return (
        <button aria-label={label} class={['copy', { copied: copied.value }]} onClick={() => copy(props.source)} title={label} type="button">
          <svg aria-hidden="true" class="glyph" fill="none" viewBox="0 0 24 24">
            <rect height="4" rx="1" width="8" x="8" y="2" />
            <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2" />
            {copied.value && <path class="copy-tick" d="m9 14 2 2 4-4" />}
          </svg>
        </button>
      )
    }
  }
})
