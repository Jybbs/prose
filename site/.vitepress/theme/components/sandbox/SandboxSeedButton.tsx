import { defineComponent } from 'vue'
import type { PropType }   from 'vue'

import { computedAsync } from '@vueuse/core'

import { seedUrl, type SavedSession } from '../../../lib/sandbox/session'

export const SandboxSeedButton = defineComponent({
  name  : 'SandboxSeedButton',
  props : {
    seed : { required: true, type: Object as PropType<SavedSession> }
  },
  setup(props) {
    // The payload deflates through `CompressionStream`, so the link resolves
    // client-side and stays empty where the platform lacks the codec.
    const href = computedAsync(
      async () => await seedUrl(props.seed.configToml, props.seed.source) ?? '', '')

    return () => (
      href.value && (
        <a
          aria-label="Open this case in the sandbox"
          class="sandbox-seed panel-corner right-40 panel-seat size-28"
          href={href.value}
          title="Open this case in the sandbox"
        >
          <svg aria-hidden="true" class="glyph" fill="none" viewBox="0 0 24 24">
            <path d="M4 9h5" />
            <path d="M14 9h6" />
            <circle cx="11.5" cy="9" r="2.2" />
            <path d="M4 15h9" />
            <path d="M18 15h2" />
            <circle cx="15.5" cy="15" r="2.2" />
          </svg>
        </a>
      )
    )
  }
})
