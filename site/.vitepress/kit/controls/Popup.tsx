import { whenever }                                                              from '@vueuse/core'
import * as hoverCard                                                            from '@zag-js/hover-card'
import { normalizeProps, useMachine }                                            from '@zag-js/vue'
import type { PropTypes }                                                        from '@zag-js/vue'
import { computed, defineComponent, h, ref, Teleport, Transition, useId, watch } from 'vue'
import type { PropType, SlotsType }                                              from 'vue'

type Placement = NonNullable<NonNullable<hoverCard.Props['positioning']>['placement']>

export interface PopupTriggerScope {
  props : PropTypes['element']
}

// Shows a popup through the `hover-card` machine, teleported to the body once
// it first opens. The `trigger` slot binds the element a pointer or a focus
// opens it from, whereas a caller delegating its own hover passes `open` and
// the `anchor` element the popup points at.
export const Popup = defineComponent({
  name         : 'Popup',
  inheritAttrs : false,
  props        : {
    anchor          : { default: undefined, type: Object as PropType<HTMLElement | null> },
    closeDelay      : { default: 120, type: Number },
    gutter          : { default: 8, type: Number },
    open            : { default: undefined, type: Boolean },
    openDelay       : { default: 80, type: Number },
    overflowPadding : { default: 8, type: Number },
    placement       : { default: 'bottom-start', type: String as PropType<Placement> },
    theme           : { required: true, type: String }
  },
  slots : Object as SlotsType<{ default: Record<string, never>, trigger: PopupTriggerScope }>,
  setup(props, { attrs, slots }) {
    const id      = useId()
    const service = useMachine(hoverCard.machine, computed(() => ({
      closeDelay  : props.closeDelay,
      id,
      open        : props.open,
      openDelay   : props.openDelay,
      positioning : {
        getAnchorRect   : props.anchor === undefined ? undefined : () => props.anchor?.getBoundingClientRect() ?? null,
        gutter          : props.gutter,
        overflowPadding : props.overflowPadding,
        placement       : props.placement
      }
    })))
    const api    = computed(() => hoverCard.connect(service, normalizeProps))
    const shown  = computed(() => props.open ?? api.value.open)
    const opened = ref(false)

    whenever(shown, () => { opened.value = true }, { flush: 'sync', immediate: true })
    watch(() => props.anchor, () => api.value.reposition())

    return () => {
      const { hidden: _hidden, ...content } = api.value.getContentProps()
      return [
        slots.trigger?.({ props: api.value.getTriggerProps() }),
        opened.value && h(Teleport, { to: 'body' }, [
          <div {...api.value.getPositionerProps()} {...attrs} data-popup={props.theme}>
            <Transition
              enterActiveClass="duration-140 ease-emphasized transition-translate-and-opacity motion-reduce:transition-none"
              enterFromClass="-translate-y-4 opacity-0"
              leaveActiveClass="duration-140 ease-emphasized transition-translate-and-opacity motion-reduce:transition-none"
              leaveToClass="-translate-y-4 opacity-0"
            >
              {shown.value && (
                <div {...content} class="z-10000">
                  {slots.default?.({})}
                  <div {...api.value.getArrowProps()}><div {...api.value.getArrowTipProps()} /></div>
                </div>
              )}
            </Transition>
          </div>
        ])
      ]
    }
  }
})
