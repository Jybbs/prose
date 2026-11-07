import { whenever }                                                              from '@vueuse/core'
import * as popover                                                              from '@zag-js/popover'
import { normalizeProps, useMachine }                                            from '@zag-js/vue'
import type { PropTypes }                                                        from '@zag-js/vue'
import { computed, defineComponent, h, ref, Teleport, Transition, useId, watch } from 'vue'
import type { PropType, SlotsType }                                              from 'vue'

type Placement = NonNullable<NonNullable<popover.Props['positioning']>['placement']>

export interface PopupTriggerScope {
  props : PropTypes['element']
}

// Clicks a trigger that is not a native button from the keys a button takes.
function pressTrigger(event: KeyboardEvent): void {
  if (event.key !== 'Enter' && event.key !== ' ') return
  event.preventDefault()
  ;(event.currentTarget as HTMLElement).click()
}

// Shows a card through the `popover` machine, teleported to the body once it
// first opens. The `trigger` slot binds the element a click, a tap, Enter, or
// Space toggles it from, focus moves into the card on opening, and Escape
// closes it and returns focus to the trigger. A caller delegating its own
// events passes `open` and the `anchor` element the card points at, and hears
// each dismissal through `dismiss`, whereas `passive` leaves focus where it is
// for a card that follows a hover.
export const Popup = defineComponent({
  name         : 'Popup',
  inheritAttrs : false,
  props        : {
    anchor          : { default: undefined, type: Object as PropType<HTMLElement | null> },
    gutter          : { default: 8, type: Number },
    label           : { required: true, type: String },
    open            : { default: undefined, type: Boolean },
    overflowPadding : { default: 8, type: Number },
    passive         : { default: false, type: Boolean },
    placement       : { default: 'bottom-start', type: String as PropType<Placement> },
    theme           : { required: true, type: String }
  },
  emits : ['dismiss'],
  slots : Object as SlotsType<{ default: Record<string, never>, trigger: PopupTriggerScope }>,
  setup(props, { attrs, emit, slots }) {
    const id       = useId()
    const anchorId = `popover:${id}:anchor-element`
    const service  = useMachine(popover.machine, computed(() => ({
      autoFocus          : !props.passive,
      finalFocusEl       : () => props.anchor ?? null,
      id,
      ids                : props.anchor === undefined || props.passive ? undefined : { trigger: anchorId },
      onOpenChange       : ({ open }: popover.OpenChangeDetails) => { if (!open) emit('dismiss') },
      open               : props.open,
      positioning        : {
        getAnchorRect   : props.anchor === undefined ? undefined : () => props.anchor?.getBoundingClientRect() ?? null,
        gutter          : props.gutter,
        overflowPadding : props.overflowPadding,
        placement       : props.placement
      },
      restoreFocus       : !props.passive
    })))
    const api    = computed(() => popover.connect(service, normalizeProps))
    const shown  = computed(() => props.open ?? api.value.open)
    const opened = ref(false)

    whenever(shown, () => { opened.value = true }, { flush: 'sync', immediate: true })
    watch(() => props.anchor, () => api.value.reposition())
    // Names the anchor as the machine's trigger, which takes Tab out of the card
    // to the tabbable after the anchor and leaves a press on the anchor to `toggle`.
    watch(() => props.passive ? null : props.anchor, (anchor, previous) => {
      previous?.removeAttribute('id')
      anchor?.setAttribute('id', anchorId)
    }, { flush: 'sync' })

    return () => {
      const { 'aria-labelledby': _named, hidden: _hidden, ...content } = api.value.getContentProps()
      const { type: _type, ...trigger }                                = api.value.getTriggerProps()
      return [
        slots.trigger?.({ props: { ...trigger, onKeydown: pressTrigger, role: 'button' } }),
        opened.value && h(Teleport, { to: 'body' }, [
          <div {...api.value.getPositionerProps()} {...attrs} data-popup={props.theme}>
            <Transition
              enterActiveClass="duration-140 ease-emphasized transition-translate-and-opacity motion-reduce:transition-none"
              enterFromClass="-translate-y-4 opacity-0"
              leaveActiveClass="duration-140 ease-emphasized transition-translate-and-opacity motion-reduce:transition-none"
              leaveToClass="-translate-y-4 opacity-0"
            >
              {shown.value && (
                <div {...content} aria-label={props.label} class="z-10000">
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
