import { defineComponent, Transition } from 'vue'
import type { PropType }               from 'vue'

import { useHiddenTabindex } from '../../../../lib/composables/use-aria-hidden'
import type { RenderedRule } from '../../../../lib/rules/rules.data'
import { familyEmblem }      from '../../../../lib/shared/emblems'
import { NibBadge }          from '../../../../kit'

export const SurfaceRailName = defineComponent({
  name  : 'SurfaceRailName',
  props : {
    rule : { required: true, type: null as unknown as PropType<RenderedRule | undefined> },
    swap : { required: true, type: String }
  },
  setup(props) {
    const tabindex = useHiddenTabindex()

    return () => {
      const { rule } = props
      return rule
        ? (
          <a
            class="surface-rail-chip chip rule-chip"
            data-family={rule.family}
            data-rule={rule.slug}
            href={rule.href}
            tabindex={tabindex.value}
          >
            <NibBadge emblem={familyEmblem(rule.family)} size={14} />
            <Transition mode="out-in" name={props.swap}>
              {{ default: () => <span class="font-mono" key={rule.slug}>{rule.slug}</span> }}
            </Transition>
          </a>
        )
        : null
    }
  }
})
