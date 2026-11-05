import * as select                                           from '@zag-js/select'
import { normalizeProps, useMachine }                        from '@zag-js/vue'
import type { PropTypes }                                    from '@zag-js/vue'
import { computed, defineComponent, useId }                  from 'vue'
import type { PropType, PublicProps, SlotsType, VNodeChild } from 'vue'

interface ListboxOption {
  id : string
}

interface ListboxProps<T extends ListboxOption> extends PublicProps {
  label                 : string
  modelValue            : string
  'onUpdate:modelValue'?: (value: string) => void
  options               : readonly T[]
}

export interface ListboxScope<T extends ListboxOption> {
  content    : PropTypes['element']
  item       : (option: T) => PropTypes['element']
  positioner : PropTypes['element']
  trigger    : PropTypes['button']
}

type GenericListbox = new <T extends ListboxOption>(props: ListboxProps<T>) => {
  $props : ListboxProps<T>
  $slots : { default: (scope: ListboxScope<T>) => VNodeChild }
}

// Binds a single-choice listbox through the `select` machine, naming both the
// trigger and the list with `label` and leaving the markup to the slot.
export const Listbox = defineComponent({
  name  : 'Listbox',
  props : {
    label      : { required: true, type: String },
    modelValue : { required: true, type: String },
    options    : { required: true, type: Array as PropType<readonly ListboxOption[]> }
  },
  emits : { 'update:modelValue': (value: string) => typeof value === 'string' },
  slots : Object as SlotsType<{ default: ListboxScope<ListboxOption> }>,
  setup(props, { emit, slots }) {
    const id         = useId()
    const collection = computed(() => select.collection({
      items        : [...props.options],
      itemToString : option => option.id,
      itemToValue  : option => option.id
    }))
    const service = useMachine(select.machine, computed(() => ({
      collection    : collection.value,
      id,
      onValueChange : ({ value }: select.ValueChangeDetails) => emit('update:modelValue', value[0]!),
      positioning   : { placement: 'bottom-start' as const, sameWidth: false },
      value         : [props.modelValue]
    })))
    const api = computed(() => select.connect(service, normalizeProps))

    return () => {
      const { 'aria-labelledby': _content, ...content } = api.value.getContentProps()
      const { 'aria-labelledby': _trigger, ...trigger } = api.value.getTriggerProps()
      return slots.default({
        content    : { ...content, 'aria-label': props.label },
        item       : option => api.value.getItemProps({ item: option }),
        positioner : api.value.getPositionerProps(),
        trigger    : { ...trigger, 'aria-label': props.label }
      })
    }
  }
}) as unknown as GenericListbox
