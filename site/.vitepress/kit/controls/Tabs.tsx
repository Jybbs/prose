import * as tabs                                             from '@zag-js/tabs'
import { mergeProps, normalizeProps, useMachine }            from '@zag-js/vue'
import type { PropTypes }                                    from '@zag-js/vue'
import { computed, defineComponent, useId }                  from 'vue'
import type { PropType, PublicProps, SlotsType, VNodeChild } from 'vue'

type Orientation = NonNullable<tabs.Props['orientation']>

interface TabsProps<T extends string> extends PublicProps {
  label                 : string
  modelValue            : T
  'onUpdate:modelValue'?: (value: T) => void
  orientation          ?: Orientation
}

export interface TabsScope<T extends string> {
  content : (value: T) => PropTypes['element']
  list    : PropTypes['element']
  trigger : (value: T) => PropTypes['button']
}

type GenericTabs = new <T extends string>(props: TabsProps<T>) => {
  $props : TabsProps<T>
  $slots : { default: (scope: TabsScope<T>) => VNodeChild }
}

// Binds a tab list through the `tabs` machine, leaving its markup to the slot,
// whose triggers each select on click, on hover, and on focus.
export const Tabs = defineComponent({
  name  : 'Tabs',
  props : {
    label       : { required: true, type: String },
    modelValue  : { required: true, type: String },
    orientation : String as PropType<Orientation>
  },
  emits : { 'update:modelValue': (value: string) => typeof value === 'string' },
  slots : Object as SlotsType<{ default: TabsScope<string> }>,
  setup(props, { emit, slots }) {
    const id      = useId()
    const service = useMachine(tabs.machine, computed(() => ({
      id,
      onValueChange : ({ value }: tabs.ValueChangeDetails) => emit('update:modelValue', value),
      orientation   : props.orientation,
      translations  : { listLabel: props.label },
      value         : props.modelValue
    })))
    const api = computed(() => tabs.connect(service, normalizeProps))

    return () => slots.default({
      content : value => api.value.getContentProps({ value }),
      list    : api.value.getListProps(),
      trigger : value => {
        const select = (): void => api.value.setValue(value)
        return mergeProps(api.value.getTriggerProps({ value }), { onFocus: select, onMouseenter: select })
      }
    })
  }
}) as unknown as GenericTabs
