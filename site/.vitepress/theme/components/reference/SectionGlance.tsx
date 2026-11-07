import { computed, defineComponent } from 'vue'
import type { PropType }             from 'vue'

import { data, type GlanceSection } from '../../../lib/reference/section-glance.data'
import { BODY_LINK_CLASS }          from '../../../lib/shared/constants'
import { InlineProse }              from '../base/InlineProse'

export const SectionGlance = defineComponent({
  name  : 'SectionGlance',
  props : {
    section : { required: true, type: null as unknown as PropType<GlanceSection> }
  },
  setup(props) {
    const entries = computed(() => data[props.section])

    return () => (
      <ul>
        {entries.value.map(entry => (
          <li key={entry.href}>
            <a class={BODY_LINK_CLASS} href={entry.href}>
              <strong>{entry.title}</strong>
            </a>
            {' '}
            <InlineProse nodes={entry.descriptionNodes} />
          </li>
        ))}
      </ul>
    )
  }
})
