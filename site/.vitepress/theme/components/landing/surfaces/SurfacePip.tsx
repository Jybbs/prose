import { defineComponent } from 'vue'
import type { PropType }   from 'vue'

import { useHiddenTabindex } from '../../../../lib/composables/use-aria-hidden'
import type { RenderedRule } from '../../../../lib/rules/rules.data'
import { formatFolio }       from '../../../../lib/shared/numerals'

export const SurfacePip = defineComponent({
  name  : 'SurfacePip',
  props : {
    active   : { required: true, type: Boolean },
    distance : { required: true, type: Number },
    index    : { required: true, type: Number },
    rule     : { required: true, type: Object as PropType<RenderedRule> }
  },
  emits : { select: () => true },
  setup(props, { emit }) {
    const tabindex = useHiddenTabindex()

    return () => (
      <a
        aria-label={props.rule.slug}
        class="surface-pip group/pip relative grid place-items-center focus-visible:rounded-pill focus-visible:outline-offset-1"
        data-active={props.active || undefined}
        href={props.rule.href}
        onFocus={() => emit('select')}
        onMouseenter={() => emit('select')}
        style={{ '--d': props.distance }}
        tabindex={tabindex.value}
      >
        <span
          aria-hidden="true"
          class="surface-pip-mark relative z-1 rounded-pill bg-family/45 group-hover/pip:bg-family-22-bg-soft group-data-active/pip:bg-family-22-bg-soft motion-reduce:delay-0"
        />
        <span class="surface-pip-folio absolute inset-0 z-2 grid shrink-0 translate-y-3 place-items-center pt-1 font-mono text-meta leading-none tracking-folio text-text-3 uppercase tabular-nums opacity-0 group-hover/pip:translate-y-0 group-hover/pip:text-family group-hover/pip:opacity-100 group-data-active/pip:translate-y-0 group-data-active/pip:text-family group-data-active/pip:opacity-100 motion-reduce:delay-0">{formatFolio(props.index + 1)}</span>
      </a>
    )
  }
})
