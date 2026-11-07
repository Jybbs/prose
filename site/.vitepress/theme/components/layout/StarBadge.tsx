import { defineComponent } from 'vue'

import { externalAttrs } from '../../../lib/shared/links'
import { data as stars } from '../../../lib/shared/stars.data'
import { REPO_URL }      from '../../../lib/shared/constants'

export const StarBadge = defineComponent({
  name: 'StarBadge',
  setup() {
    return () => (
      <a
        href={REPO_URL}
        title="GitHub stars"
        {...externalAttrs(REPO_URL)}
        class="ms-6 inline-flex items-center gap-4 border-1 border-divider px-8 py-2 font-mono text-label leading-none tracking-folio text-text-2"
      >
        <span aria-hidden="true" class="text-brand-1">★</span>
        {' '}
        {stars.stars}
      </a>
    )
  }
})
