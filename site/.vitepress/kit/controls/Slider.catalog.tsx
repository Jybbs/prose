import { defineComponent, ref, useId } from 'vue'

import type { Declaration } from '../catalog'
import { Slider }           from './Slider'
import type { SliderScope } from './Slider'

const MAX = 130
const MIN = 60

const Example = defineComponent(() => {
  const track = useId()
  const value = ref(88)

  return () => (
    <div class="relative h-28 border-b-1 border-b-divider" id={track}>
      <Slider control={track} label="Line length" max={MAX} min={MIN} modelValue={value.value} onChange={next => { value.value = next }}>
        {{
          default: ({ thumb }: SliderScope) => (
            <div
              {...thumb}
              class="absolute bottom-0 -translate-x-1/2 cursor-grab rounded-sm border-1 border-divider bg-bg px-5 font-mono text-0-68rem"
              style={{ left: `${(value.value - MIN) / (MAX - MIN) * 100}%` }}
            >{value.value}</div>
          )
        }}
      </Slider>
    </div>
  )
})

export const declaration: Declaration = {
  examples : [
    { label: 'Drag the thumb, or use the arrow keys', render: () => <Example /> }
  ],
  use      : 'Binds one thumb to a value on a track the caller draws, so several thumbs can share one track.'
}
