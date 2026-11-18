import { defineComponent, Transition } from 'vue'
import type { PropType }               from 'vue'

import { useHiddenTabindex } from '../../../../lib/composables/use-aria-hidden'
import type { RenderedRule } from '../../../../lib/rules/rules.data'
import { familyEmblem }      from '../../../../lib/shared/emblems'
import { NibBadge }          from '../../../../kit'

type Swap = keyof typeof SWAP_CLASSES

// Names the classes that slide a rule's name in from the side it travels from
// and out toward the side it travels to.
const SWAP_CLASSES = {
  back : { enterFromClass: 'opacity-0 -translate-x-5', leaveToClass: 'opacity-0 translate-x-5' },
  fwd  : { enterFromClass: 'opacity-0 translate-x-5', leaveToClass: 'opacity-0 -translate-x-5' }
}

export const SurfaceRailName = defineComponent({
  name  : 'SurfaceRailName',
  props : {
    rule : { required: true, type: null as unknown as PropType<RenderedRule | undefined> },
    swap : { required: true, type: String as PropType<Swap> }
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
            <Transition
              {...SWAP_CLASSES[props.swap]}
              enterActiveClass="surface-rail-swap"
              leaveActiveClass="surface-rail-swap"
              mode="out-in"
            >
              {{ default: () => <span class="font-mono" key={rule.slug}>{rule.slug}</span> }}
            </Transition>
          </a>
        )
        : null
    }
  }
})
