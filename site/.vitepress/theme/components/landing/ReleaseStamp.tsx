import { defineComponent } from 'vue'
import type { PropType }   from 'vue'

import type { PyPIRelease } from '../../../lib/landing/pypi-releases.data'
import { externalAttrs }    from '../../../lib/shared/links'

export const ReleaseStamp = defineComponent({
  name  : 'ReleaseStamp',
  props : {
    release : { required: true, type: Object as PropType<PyPIRelease> }
  },
  setup(props) {
    return () => (
      <a
        href={props.release.url}
        {...externalAttrs(props.release.url)}
        class="relative flex size-132 -rotate-6 flex-col items-center justify-center gap-2 rounded-half border-2 border-dashed border-stamp-accent-65-panel-edge bg-stamp-accent-12-panel-bg text-center font-mono text-stamp-ink transition-transform hover:scale-104 hover:-rotate-3"
        title={`Current release ${props.release.version}, ${props.release.month} ${props.release.year}`}
      >
        <span
          aria-hidden="true"
          class="pointer-events-none absolute inset-8 rounded-half border-1 border-stamp-accent-40-panel-edge"
        />
        <span class="text-xxs tracking-display-wide text-stamp-accent-55-text-3">{props.release.month}</span>
        <span class="font-display text-1-3rem leading-none italic">{props.release.version}</span>
        <span class="text-0-74rem tracking-0-16em text-stamp-accent-50-text-3">'{props.release.yearShort}</span>
      </a>
    )
  }
})
