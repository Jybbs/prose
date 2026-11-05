import * as slider                          from '@zag-js/slider'
import { normalizeProps, useMachine }       from '@zag-js/vue'
import type { PropTypes }                   from '@zag-js/vue'
import { computed, defineComponent, useId } from 'vue'
import type { SlotsType }                   from 'vue'

export interface SliderScope {
  thumb : Omit<PropTypes['element'], 'aria-labelledby' | 'style'>
}

// Binds one thumb to the `slider` machine against the track element `control`
// names, so thumbs on one track hold independent values and may cross. The
// slot places the thumb, and a press that moves it reports through `change`
// before `commit` reports the release.
export const Slider = defineComponent({
  name  : 'Slider',
  props : {
    control    : { required: true, type: String },
    label      : { required: true, type: String },
    max        : { required: true, type: Number },
    min        : { required: true, type: Number },
    modelValue : { required: true, type: Number }
  },
  emits : {
    change : (value: number) => typeof value === 'number',
    commit : () => true
  },
  slots : Object as SlotsType<{ default: SliderScope }>,
  setup(props, { emit, slots }) {
    const id      = useId()
    const service = useMachine(slider.machine, computed(() => ({
      'aria-label'     : [props.label],
      id,
      ids              : { control: props.control },
      max              : props.max,
      min              : props.min,
      onValueChange    : ({ value }: slider.ValueChangeDetails) => emit('change', value[0]!),
      onValueChangeEnd : () => emit('commit'),
      thumbAlignment   : 'center' as const,
      value            : [props.modelValue]
    })))
    const api = computed(() => slider.connect(service, normalizeProps))

    return () => {
      const { 'aria-labelledby': _labelledBy, style: _style, ...thumb } = api.value.getThumbProps({ index: 0 })
      return slots.default({ thumb })
    }
  }
})
