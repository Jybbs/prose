import { computed, defineComponent, useId } from 'vue'
import type { PropType }                    from 'vue'

import type { Emblem as Motif } from '../../.vitepress/lib/shared/emblems'
import * as emblems             from './emblems'

const TIP_FILLS = { cut: '#000', ink: 'currentColor' } as const

export const Emblem = defineComponent({
  name  : 'Emblem',
  props : {
    emblem     : { required: true, type: Object as PropType<Motif> },
    house      : { required: true, type: null as unknown as PropType<emblems.House> },
    standalone : Boolean
  },
  setup(props) {
    const id      = useId()
    const nib     = computed(() => props.house.terminal === 'nib')
    const line    = computed(() => props.house.form === 'line')
    const painted = computed(() => !line.value || props.house.tone === 'duotone')

    const tips = (path: string, marker: keyof typeof TIP_FILLS): Record<string, string> => {
      if (!nib.value || emblems.isClosed(path)) return {}
      const url = `url(#${id}-${marker})`
      return { 'marker-end': url, 'marker-start': url }
    }

    const strokes = computed(() => {
      const { cuts, lines, shapes } = props.emblem.motif
      return line.value ? [...shapes, ...cuts, ...lines] : lines
    })

    return () => (
      <svg
        aria-hidden={props.standalone ? undefined : 'true'}
        aria-label={props.standalone ? props.emblem.motif.label : undefined}
        class="icon-emblem glyph text-emblem-ink"
        data-family={props.emblem.family}
        data-form={props.house.form}
        data-terminal={props.house.terminal}
        data-tone={props.house.tone}
        fill="none"
        role={props.standalone ? 'img' : undefined}
        style={{
          '--emblem-family' : props.emblem.family && `var(--color-family-${props.emblem.family})`,
          strokeWidth       : props.house.weight
        }}
        viewBox="0 0 24 24"
      >
        <defs>
          {nib.value && (
            Object.entries(TIP_FILLS).map(([tip, fill]) => (
              <marker
                id={`${id}-${tip}`}
                key={tip}
                markerHeight="1"
                markerUnits="strokeWidth"
                markerWidth="0.55"
                orient="auto-start-reverse"
                viewBox="-0.05 -0.5 0.55 1"
              >
                <path d="M-0.05-0.5 0.5 0-0.05 0.5Z" fill={fill} stroke="none" />
              </marker>
            ))
          )}
          {painted.value && (
            <mask id={`${id}-mask`}>
              <rect fill="#fff" height="24" stroke="none" width="24" />
              {props.emblem.motif.cuts.map(cut => <path d={cut} fill={emblems.isClosed(cut) ? '#000' : 'none'} key={cut} stroke="#000" {...tips(cut, 'cut')} />)}
            </mask>
          )}
        </defs>
        {painted.value && (
          <g class="icon-emblem-paint fill-emblem-paint stroke-none" mask={`url(#${id}-mask)`}>
            {props.emblem.motif.shapes.map(shape => <path d={shape} key={shape} />)}
          </g>
        )}
        {!line.value && (
          <g class="icon-emblem-outline stroke-emblem-paint">
            {props.emblem.motif.shapes.map(shape => <path d={shape} key={shape} />)}
          </g>
        )}
        {strokes.value.map(stroke => <path d={stroke} key={stroke} {...tips(stroke, 'ink')} />)}
      </svg>
    )
  }
})
