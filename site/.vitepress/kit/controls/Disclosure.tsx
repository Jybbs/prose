import * as collapsible                     from '@zag-js/collapsible'
import { normalizeProps, useMachine }       from '@zag-js/vue'
import type { PropTypes }                   from '@zag-js/vue'
import { computed, defineComponent, useId } from 'vue'
import type { SlotsType }                   from 'vue'

export interface DisclosureScope {
  content : PropTypes['element']
  trigger : PropTypes['button']
}

// Binds one disclosure through the `collapsible` machine, leaving its markup
// to the slot and its open state to the caller.
export const Disclosure = defineComponent({
  name  : 'Disclosure',
  props : { open: { required: true, type: Boolean } },
  emits : { 'update:open': (open: boolean) => typeof open === 'boolean' },
  slots : Object as SlotsType<{ default: DisclosureScope }>,
  setup(props, { emit, slots }) {
    const id      = useId()
    const service = useMachine(collapsible.machine, computed(() => ({
      id,
      onOpenChange : ({ open }: collapsible.OpenChangeDetails) => emit('update:open', open),
      open         : props.open
    })))
    const api = computed(() => collapsible.connect(service, normalizeProps))

    return () => slots.default({
      content : api.value.getContentProps(),
      trigger : api.value.getTriggerProps()
    })
  }
})
