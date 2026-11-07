import { useElementSize, useResizeObserver }                                                    from '@vueuse/core'
import { computed, defineComponent, reactive, ref, shallowRef, useId, withKeys, withModifiers } from 'vue'
import type { CSSProperties, PropType }                                                         from 'vue'

import { Slider }           from '../../../kit'
import type { SliderScope } from '../../../kit'

import type { LengthKnob } from '../../../lib/sandbox/config-schema.data'
import * as rulerScale     from '../../../lib/sandbox/ruler-scale'
import { keyHandlers }     from '../../../lib/shared/key-handlers'

const MAX  = rulerScale.LENGTH_MAX

const MIN  = rulerScale.LENGTH_MIN

const SPAN = MAX - MIN

const DRAG_SLOP = 3

const HUES      = ['ube', 'dexter', 'whiskey'] as const

const LANE_SLOP = 6

const MARKS     = [MIN, 80, 130, MAX] as const

export const SandboxRuler = defineComponent({
  name  : 'SandboxRuler',
  props : {
    lengths  : { required: true, type: Array as PropType<readonly LengthKnob[]> },
    lengthOf : { required: true, type: Function as PropType<(key: string) => number> }
  },
  emits : {
    dragging  : (key: string, hue: string) => typeof key === 'string' && typeof hue === 'string',
    preview   : (key: string, value: number) => typeof key === 'string' && typeof value === 'number',
    setLength : (key: string, value: number) => typeof key === 'string' && typeof value === 'number'
  },
  setup(props, { emit }) {
    const chipEls = shallowRef<HTMLElement[] | null>(null)
    const chipWidths = reactive(new Map<string, number>())
    const drag       = ref<{ key: string, start: number } | null>(null)
    const draft      = ref('')
    const editing    = ref('')
    const preview    = ref<{ key: string, value: number } | null>(null)
    const trackEl    = ref<HTMLElement | null>(null)
    const trackId    = useId()

    // A drag moves this preview alone, the config committing on release, so the
    // code and toml surfaces hold still while a stop is in hand.
    function shownValue(knob: LengthKnob): number {
      const held = preview.value
      return held && held.key === knob.key ? held.value : props.lengthOf(knob.key)
    }

    const { width: trackWidth } = useElementSize(trackEl)

    useResizeObserver(chipEls, entries => {
      for (const { target } of entries) {
        const chip = target as HTMLElement
        if (chip.dataset.key !== undefined) chipWidths.set(chip.dataset.key, chip.offsetWidth)
      }
    })

    const tierByKey = computed(() => rulerScale.packTiers(
      props.lengths.map(knob => ({ key: knob.key, pct: rulerScale.pctOfLength(shownValue(knob)) })),
      trackWidth.value,
      chipWidths
    ))

    // Joins each knob's default to its value, a tether that would overlap another
    // dropping to the next lane.
    const tethers = computed(() => rulerScale.packLanes(
      new Map(props.lengths.map(knob => {
        const [at, home] = [shownValue(knob), knob.default].map(rulerScale.pctOfLength) as [number, number]
        return [knob.key, { from: Math.min(at, home), to: Math.max(at, home) }]
      })),
      trackWidth.value,
      LANE_SLOP
    ))

    // The head reserves rows only for tiers in use, so it stays one row tall
    // until chips actually crowd.
    const rows = computed(() => {
      let top = 0
      for (const tier of tierByKey.value.values()) top = Math.max(top, tier)
      return top + 1
    })

    function beginEdit(knob: LengthKnob): void {
      drag.value    = null
      draft.value   = String(props.lengthOf(knob.key))
      editing.value = knob.key
    }

    function cancelEdit(): void {
      editing.value = ''
    }

    function commitEdit(knob: LengthKnob): void {
      if (editing.value !== knob.key) return
      const parsed = Number.parseInt(draft.value, 10)
      if (Number.isFinite(parsed)) emit('setLength', knob.key, rulerScale.clampLength(parsed))
      editing.value = ''
    }

    function dragEnd(): void {
      if (!drag.value) return
      const held = preview.value
      if (held) emit('setLength', held.key, held.value)
      preview.value = null
      drag.value    = null
      emit('dragging', '', '')
    }

    // Commits a value no press holds, as a key step reports one. Under a press it
    // holds the stop still until the value it would take sits `DRAG_SLOP` pixels
    // from where the press began, so a double-click edits rather than moves.
    function dragMove(knob: LengthKnob, value: number): void {
      const state = drag.value
      if (!state) {
        emit('setLength', knob.key, value)
        return
      }
      if (state.key !== knob.key) return
      if (!preview.value && Math.abs(value - state.start) * trackWidth.value / SPAN < DRAG_SLOP) return
      preview.value = { key: knob.key, value }
      emit('preview', knob.key, value)
    }

    function dragStart(hue: string, knob: LengthKnob, event: PointerEvent): void {
      if (editing.value === knob.key || event.button !== 0) return
      emit('dragging', knob.key, hue)
      drag.value = { key: knob.key, start: props.lengthOf(knob.key) }
    }

    // Gives each knob its hue by its place, so a knob, its default, its tether,
    // and the guide it holds share one.
    function hueOf(index: number): string {
      return HUES[index % HUES.length]!
    }

    function focusInput(el: unknown): void {
      if (el instanceof HTMLInputElement && el !== document.activeElement) el.select()
    }

    function stopStyle(knob: LengthKnob): CSSProperties {
      return {
        '--pct'  : rulerScale.pctOfLength(shownValue(knob)),
        '--tier' : tierOf(knob.key)
      }
    }

    function tetherStyle(knob: LengthKnob): CSSProperties & { '--lane': number } {
      const { from, lane, to } = tethers.value.get(knob.key)!
      return { '--lane': lane, 'left': `${from}%`, 'width': `${to - from}%` }
    }

    function tierOf(key: string): number {
      return tierByKey.value.get(key) ?? 0
    }

    return () => (
      <div class="flex flex-wrap items-center gap-x-1-25rem gap-y-0-75rem">
        <div
          aria-label="Line lengths"
          class="sandbox-ruler relative min-w-0 select-none"
          id={trackId}
          ref={trackEl}
          role="group"
          style={{ '--rows': rows.value }}
        >
          <div aria-hidden="true" class="sandbox-ruler-track absolute" />
          {props.lengths.map((knob, index) => (
            <span
              aria-hidden="true"
              class="sandbox-ruler-tether pointer-events-none absolute h-2"
              data-hue={hueOf(index)}
              key={`tether-${knob.key}`}
              style={tetherStyle(knob)}
            />
          ))}
          {props.lengths.map((knob, index) => (
            <span
              aria-hidden="true"
              class="sandbox-ruler-default absolute w-6 h-6 bg-bg-soft"
              data-hue={hueOf(index)}
              key={`default-${knob.key}`}
              style={{ '--lane': tetherStyle(knob)['--lane'], 'left': `${rulerScale.pctOfLength(knob.default)}%` }}
            />
          ))}
          {MARKS.map(mark => (
            <span
              aria-hidden="true"
              class="sandbox-ruler-num absolute text-text-3 font-mono text-0-6rem leading-none"
              key={mark}
              style={{ left: `${rulerScale.pctOfLength(mark)}%` }}
            >{mark}</span>
          ))}
          {props.lengths.map((knob, index) => (
            <Slider
              control={trackId}
              key={knob.key}
              label={`${knob.label} line length`}
              max={MAX}
              min={MIN}
              modelValue={shownValue(knob)}
              onChange={event => dragMove(knob, event)}
              onCommit={dragEnd}
            >
              {{
                default: ({ thumb }: SliderScope) => (
                  <div
                    class="sandbox-ruler-stop group focus-within:z-2 hover:z-2 data-dragging:z-2 absolute w-0 cursor-ew-resize touch-none select-none"
                    data-changed={shownValue(knob) !== knob.default || null}
                    data-hue={hueOf(index)}
                    onPointercancel={dragEnd}
                    onPointerdown={event => dragStart(hueOf(index), knob, event)}
                    {...editing.value === knob.key ? null : thumb}
                    data-dragging={drag.value?.key === knob.key || null}
                    style={stopStyle(knob)}
                  >
                    <span
                      aria-hidden="true"
                      class="sandbox-ruler-marker before:absolute before:inset-x-4 before:inset-y-0 before:transition-colors before:bg-text-2 group-hover:before:bg-text-1 group-focus-visible:before:bg-text-1 group-data-dragging:before:bg-text-1 absolute w-10"
                    />
                    {editing.value !== knob.key
                      ? (
                        <span
                          class="sandbox-ruler-chip px-7 group-focus-visible:focus-ring group-focus-visible:outline-offset-1 group-hover:border-text-2 group-data-dragging:border-text-2 absolute z-1 inline-flex gap-0-4rem items-center py-0 border-1 border-divider rounded-sm bg-bg leading-none whitespace-nowrap"
                          data-key={knob.key}
                          onDblclick={() => beginEdit(knob)}
                          ref={chipEls}
                          ref_for
                        >
                          <span class="font-mono text-meta tracking-0-05em text-text-2 uppercase">{knob.label}</span>
                          <span class="font-mono text-0-68rem text-text-1 tabular-nums transition-colors group-data-changed:text-knob-figure">{shownValue(knob)}</span>
                        </span>
                      )
                      : (
                      <input
                        aria-label={`${knob.label} line length`}
                        class="sandbox-ruler-chip sandbox-ruler-chip-input panel-number w-3-4rem cursor-text px-5 font-mono text-0-68rem text-text-1 select-text focus-visible:-outline-offset-1 group-hover:border-text-2 group-data-dragging:border-text-2 absolute z-1 inline-flex gap-0-4rem items-center py-0 border-1 border-divider rounded-sm bg-bg leading-none whitespace-nowrap"
                        max={MAX}
                        min={MIN}
                        onBlur={() => commitEdit(knob)}
                        onKeydown={keyHandlers(
                          event => event.stopPropagation(),
                          withKeys(withModifiers(() => commitEdit(knob), ['prevent']), ['enter']),
                          withKeys(cancelEdit, ['esc'])
                        )}
                        ref={focusInput}
                        type="number"
                        v-model={draft.value}
                      />
                    )}
                  </div>
                )
              }}
            </Slider>
          ))}
        </div>
      </div>
    )
  }
})
