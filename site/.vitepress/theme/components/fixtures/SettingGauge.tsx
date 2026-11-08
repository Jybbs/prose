import { computed, defineComponent, useId, useModel } from 'vue'
import type { PropType }                             from 'vue'

import { Slider }           from '../../../kit'
import type { SliderScope } from '../../../kit'

import type { FixtureGauge } from '../../../lib/fixtures/entry'
import * as gauge            from '../../../lib/fixtures/gauge'

// Steps one numeric setting along a ruler, a taller tick standing at each value
// where the case's output changes and a hollow diamond at the setting's default.
export const SettingGauge = defineComponent({
  name  : 'SettingGauge',
  props : {
    gauge      : { required: true, type: Object as PropType<FixtureGauge> },
    modelValue : { required: true, type: Number }
  },
  emits : { 'update:modelValue': (value: number) => typeof value === 'number' },
  setup(props) {
    const trackId = useId()
    const value   = useModel(props, 'modelValue')

    const last  = computed(() => props.gauge.steps.length - 1)
    const marks = computed(() => [0, props.gauge.default, last.value])
    const ticks = computed(() => gauge.gaugeTicks(props.gauge.steps))

    const pct = (at: number): string => `${gauge.gaugePct(props.gauge.steps, at)}%`

    return () => (
      <div
        aria-label={`${props.gauge.setting} values`}
        class="fixture-gauge relative mb-1rem select-none"
        id={trackId}
        role="group"
      >
        <span class="fixture-gauge-ticks absolute inset-x-0 text-text-3" innerHTML={ticks.value} />
        <span
          aria-hidden="true"
          class="fixture-gauge-default absolute size-6 bg-bg-soft"
          style={{ left: pct(props.gauge.default) }}
        />
        {marks.value.map((mark, at) => (
          <span
            aria-hidden="true"
            class="fixture-gauge-num absolute font-mono text-0-6rem leading-none text-text-3"
            data-end={at === 0 ? 'start' : at === marks.value.length - 1 ? 'end' : undefined}
            key={mark}
            style={{ left: pct(mark) }}
          >{mark}</span>
        ))}
        <Slider
          control={trackId}
          label={props.gauge.setting}
          max={last.value}
          min={0}
          modelValue={value.value}
          onChange={next => { value.value = next }}
        >
          {{
            default: ({ thumb }: SliderScope) => (
              <div
                {...thumb}
                class="fixture-gauge-stop group absolute w-0 cursor-ew-resize touch-none outline-none"
                style={{ '--pct': gauge.gaugePct(props.gauge.steps, value.value) }}
              >
                <span aria-hidden="true" class="fixture-gauge-marker absolute bg-text-2 group-hover:bg-text-1" />
                <span class="fixture-gauge-chip ruler-chip group-focus-visible:focus-ring absolute group-hover:border-text-2">
                  <span class="font-mono text-meta tracking-0-05em text-text-2">{props.gauge.setting}</span>
                  <span class="font-mono text-0-68rem tabular-nums text-text-1">{value.value}</span>
                </span>
              </div>
            )
          }}
        </Slider>
      </div>
    )
  }
})
