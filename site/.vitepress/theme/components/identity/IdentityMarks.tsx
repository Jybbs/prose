import { clamp, defaultWindow, useEventListener } from '@vueuse/core'
import { computed, defineComponent, nextTick, onMounted, shallowRef, useId, watch, watchEffect } from 'vue'

import { Slider }                                       from '../../../kit'
import type { SliderScope }                             from '../../../kit'
import { data }                                         from '../../../lib/identity/identity.data'
import * as marks                                       from '../../../lib/identity/marks'
import { SITE_TAGLINE }                                 from '../../../lib/shared/constants'
import { IdentityCaption, IdentityCopy, IdentityPanel } from './IdentityParts'

const LOCKUP = data.marks.files['title-with-tagline.svg']

type Side = 'bottom' | 'left' | 'right' | 'top'

// Sets the clear-space labels at this many pixels on screen, this far from
// the line each one names.
const LABEL_PX = 10

const LABEL_GAP = 4

// Writes a drawing as an `<svg>` set at `height` pixels, its width following
// its box.
const svgOf = (drawing: marks.Drawing, height: number): string =>
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${drawing.viewBox}" height="${height}" aria-hidden="true">${drawing.markup}</svg>`

const units = (value: number): string => `${Math.round(value * 10) / 10}`

const LIST = new Intl.ListFormat('en', { type: 'conjunction' })

// Draws the parts the switches turn on, of the pen-nib P, the letters, and the
// tagline, on a stage over the lockup's box, marking the clear space they
// leave on each side in the file's own units. The same parts sit in a copy of
// the site's nav title, whose own name hides while the letters draw it, and a
// handle at the mark's corner or a ruler beneath it resizes the copy, the
// ruler marking the height the nav bar serves the mark at. The rows under it
// name the file that draws the parts turned on and the tagline the lockup
// sets.
export const IdentityMarks = defineComponent({
  name : 'IdentityMarks',
  setup() {
    const on       = shallowRef<marks.PartId[]>(marks.PARTS.map(part => part.id))
    const height   = shallowRef(0)
    const served   = shallowRef(0)
    const range    = shallowRef({ max: 0, min: 0 })
    const ink      = shallowRef<Record<Side, number> | null>(null)
    const scale    = shallowRef(1)
    const art      = shallowRef<SVGGElement | null>(null)
    const stage    = shallowRef<HTMLElement | null>(null)
    const slot     = shallowRef<HTMLElement | null>(null)
    const wordmark = shallowRef<HTMLElement | null>(null)
    const corner   = shallowRef({ left: 0, top: 0 })
    const trackId  = useId()
    const file     = computed(() => marks.fileDrawing(on.value))
    const drawing  = computed((): marks.Drawing => file.value
      ? data.marks.files[file.value]
      : { markup: on.value.map(id => data.marks.parts[id].markup).join(''), viewBox: LOCKUP.viewBox })
    const box      = computed(() => {
      const [x, y, w, h] = drawing.value.viewBox.split(' ').map(Number) as [number, number, number, number]
      return { height: h, width: w, x, y }
    })
    const percent  = (px: number): number => (px - range.value.min) / (range.value.max - range.value.min) * 100
    const composed = computed(() => on.value.length > 0)
    const wide     = computed(() => Math.round(height.value * box.value.width / box.value.height))

    const toggle = (id: marks.PartId): void => {
      on.value = marks.PARTS.map(part => part.id).filter(each => (each === id) !== on.value.includes(each))
    }

    const measure = (): boolean => {
      const logo = document.querySelector<HTMLElement>('.VPNavBarTitle .logo')
      const bar  = document.querySelector<HTMLElement>('.VPNavBar')
      if (!logo || !bar) return false
      served.value = logo.getBoundingClientRect().height
      range.value  = { max: bar.getBoundingClientRect().height, min: served.value / 2 }
      height.value = clamp(height.value || served.value, range.value.min, range.value.max)
      return true
    }

    // Reads how many screen pixels one unit of the stage's drawing takes.
    const rescale = (): void => { scale.value = art.value?.getScreenCTM?.()?.a || 1 }

    useEventListener(defaultWindow, 'resize', () => {
      measure()
      rescale()
    })

    onMounted(() => {
      const title = document.querySelector<HTMLElement>('.VPNavBarTitle')
      const logo  = title?.querySelector<HTMLElement>('.logo')
      if (!title || !logo || !measure()) return
      const copy   = title.cloneNode(true) as HTMLElement
      const holder = document.createElement('span')
      for (const { name, value } of logo.attributes) if (name.startsWith('data-v-') || name === 'class') holder.setAttribute(name, value)
      copy.querySelector('.logo')!.replaceWith(holder)
      stage.value!.prepend(copy)
      wordmark.value = holder.nextElementSibling as HTMLElement | null
      slot.value     = holder
    })

    watchEffect(() => {
      if (!slot.value) return
      slot.value.innerHTML = composed.value ? svgOf(drawing.value, Math.round(height.value)) : ''
      wordmark.value?.toggleAttribute('hidden', on.value.includes('letters'))
      const mark  = slot.value.getBoundingClientRect()
      const frame = stage.value!.getBoundingClientRect()
      corner.value = { left: mark.right - frame.left, top: mark.bottom - frame.top }
    })

    watch([drawing, art], async () => {
      await nextTick()
      const bounds = composed.value && art.value && 'getBBox' in art.value ? art.value.getBBox() : null
      const { height: h, width: w, x, y } = box.value
      rescale()
      ink.value = bounds && {
        bottom : y + h - bounds.y - bounds.height,
        left   : bounds.x - x,
        right  : x + w - bounds.x - bounds.width,
        top    : bounds.y - y
      }
    }, { immediate: true })

    const drag = (event: PointerEvent, from: (move: PointerEvent) => number): void => {
      const target = event.currentTarget as HTMLElement
      target.setPointerCapture(event.pointerId)
      const move = (next: PointerEvent): void => {
        height.value = clamp(from(next), range.value.min, range.value.max)
      }
      move(event)
      target.addEventListener('pointermove', move)
      target.addEventListener('pointerup', () => { target.removeEventListener('pointermove', move) }, { once: true })
    }

    const fromHandle = (event: PointerEvent): void => {
      const start = { height: height.value, y: event.clientY }
      drag(event, next => start.height + next.clientY - start.y)
    }

    const fromTrack = (event: PointerEvent): void => {
      const track = (event.currentTarget as HTMLElement).getBoundingClientRect()
      drag(event, next => range.value.min + (next.clientX - track.left) / track.width * (range.value.max - range.value.min))
    }

    // Draws the file's box, the box the ink fills, and a line across the clear
    // space on each side between the two, labeled with its width at a size
    // that stays the same on screen whatever the file's units.
    const guides = (space: Record<Side, number>) => {
      const { height: h, width: w, x, y } = box.value
      const inner  = { height: h - space.top - space.bottom, width: w - space.left - space.right, x: x + space.left, y: y + space.top }
      const middle = { x: inner.x + inner.width / 2, y: inner.y + inner.height / 2 }
      const gap    = LABEL_GAP / scale.value
      const labels = [
        { side: 'left',   x: x + space.left / 2,                      y: middle.y - gap },
        { side: 'right',  x: inner.x + inner.width + space.right / 2, y: middle.y - gap },
        { side: 'top',    x: middle.x + gap,                          y: y + space.top / 2 },
        { side: 'bottom', x: middle.x + gap,                          y: inner.y + inner.height + space.bottom / 2 }
      ] as const
      return (
        <g aria-hidden="true" class="identity-guides" font-size={LABEL_PX / scale.value}>
          <rect height={h} width={w} x={x} y={y} />
          <rect class="identity-guide-ink" {...inner} />
          <line x1={x} x2={inner.x} y1={middle.y} y2={middle.y} />
          <line x1={inner.x + inner.width} x2={x + w} y1={middle.y} y2={middle.y} />
          <line x1={middle.x} x2={middle.x} y1={y} y2={inner.y} />
          <line x1={middle.x} x2={middle.x} y1={inner.y + inner.height} y2={y + h} />
          {labels.map(label => (
            <text data-side={label.side} key={label.side} x={label.x} y={label.y}>{units(space[label.side])}</text>
          ))}
        </g>
      )
    }

    const sentence = computed(() => {
      const space = ink.value
      if (!space) return ''
      const parts  = `the ${LIST.format(marks.PARTS.filter(part => on.value.includes(part.id)).map(part => part.label.replace(/^\w/, first => first.toLowerCase())))}`
      const leaves = `leaving ${units(space.left)} at the left, ${units(space.right)} at the right, ${units(space.top)} above, and ${units(space.bottom)} below, in the file’s own units`
      return file.value
        ? `${file.value} sets ${parts} in a ${box.value.width} by ${box.value.height} box, ${leaves}.`
        : `The lockup’s ${box.value.width} by ${box.value.height} box holds ${parts}, ${leaves}.`
    })

    return () => (
      <div>
        <IdentityPanel source={file.value ?? 'no single file'} title="Lockup">
          {{
            aside   : () => <span class="kicker">{`${marks.PARTS.length} parts · ${Object.keys(marks.MARK_FILES).length} files`}</span>,
            default : () => (
              <div class="grid gap-14 px-18 pt-14 pb-18">
                <div class="flex flex-wrap gap-x-1-5rem gap-y-0-75rem">
                  {marks.PARTS.map(part => (
                    <span class="inline-flex items-center gap-8" key={part.id}>
                      <button
                        aria-checked={on.value.includes(part.id)}
                        aria-label={part.label}
                        class="group/switch relative h-22 w-40 cursor-pointer rounded-pill border-none bg-text-3/40 transition-colors focus-visible:focus-ring focus-visible:outline-offset-1 aria-checked:bg-text-1"
                        onClick={() => toggle(part.id)}
                        role="switch"
                        type="button"
                      >
                        <span class="absolute top-3 left-3 size-16 rounded-half bg-bg transition-transform group-aria-checked/switch:translate-x-18" />
                      </button>
                      <span aria-hidden="true" class="kicker">{part.label}</span>
                    </span>
                  ))}
                </div>
                <div class="relative rounded-sm border-1 border-divider bg-bg px-1-5rem py-1-25rem">
                  <svg aria-hidden="true" class="identity-stage-art mx-auto block overflow-visible" viewBox={drawing.value.viewBox}>
                    {ink.value && guides(ink.value)}
                    <g innerHTML={composed.value ? drawing.value.markup : ''} ref={art} />
                  </svg>
                </div>
                <div class="flex flex-wrap items-baseline justify-between gap-x-12 gap-y-4">
                  <span class="kicker">In the nav bar</span>
                  <span class="text-xs text-text-2">
                    {composed.value && <span class="font-mono text-text-1 tabular-nums">{`${Math.round(height.value)} px tall · ${wide.value} px wide`}</span>}
                    {composed.value && !file.value && <em class="ml-8">no file under site/public/ draws these parts alone</em>}
                  </span>
                </div>
                <div class="relative">
                  <div class="identity-nav flex items-center overflow-hidden rounded-sm border-1 border-divider bg-bg px-1-25rem" inert ref={stage} />
                  {served.value > 0 && composed.value && (
                    <span
                      aria-hidden="true"
                      class="absolute size-14 -translate-1/2 cursor-nwse-resize touch-none rounded-sm border-1 border-text-2 bg-bg"
                      onPointerdown={fromHandle}
                      style={{ left: `${corner.value.left}px`, top: `${corner.value.top}px` }}
                    />
                  )}
                </div>
                {served.value > 0 && (
                  <div class="grid gap-6">
                    <div
                      aria-label="Mark height"
                      class="relative h-2-1rem cursor-ew-resize touch-none select-none"
                      id={trackId}
                      onPointerdown={fromTrack}
                      role="group"
                    >
                      <span aria-hidden="true" class="identity-ruler-track absolute inset-x-0" />
                      <span
                        aria-hidden="true"
                        class="identity-diamond absolute"
                        style={{ left: `${percent(served.value)}%` }}
                        title="The nav bar's served height"
                      />
                      <Slider
                        control={trackId}
                        label="Mark height in pixels"
                        max={range.value.max}
                        min={range.value.min}
                        modelValue={height.value}
                        onChange={(next: number) => { height.value = next }}
                      >
                        {{
                          default: ({ thumb }: SliderScope) => (
                            <span
                              {...thumb}
                              class="absolute top-1/2 size-14 -translate-1/2 rounded-pill bg-text-1 focus-visible:focus-ring focus-visible:outline-offset-1"
                              style={{ left: `${percent(height.value)}%` }}
                            />
                          )
                        }}
                      </Slider>
                    </div>
                    <p class="m-0 flex justify-between kicker tabular-nums">
                      <span>{`${Math.round(range.value.min)} px`}</span>
                      <span>{`${Math.round(range.value.max)} px · the bar’s height`}</span>
                    </p>
                  </div>
                )}
                <dl aria-live="polite" class="m-0 grid grid-cols-max-1fr items-baseline gap-x-1-25rem">
                  <dt class="kicker">Matches</dt>
                  <dd class="m-0 flex flex-wrap items-baseline gap-x-10 gap-y-4 pb-10">
                    {file.value
                      ? [
                          <span class="font-display text-1rem font-500 text-text-1 italic" key="name">{file.value}</span>,
                          <IdentityCopy key="path" label={`/${file.value}`} value={`/${file.value}`} />,
                          <span class="font-mono text-meta text-text-3" key="box">{`viewBox ${data.marks.files[file.value].viewBox}`}</span>
                        ]
                      : <span class="text-sm text-text-2 italic">{composed.value ? 'No served file draws these parts alone.' : 'Turn a part on to compose a mark.'}</span>}
                  </dd>
                  <dt class="border-t-1 border-t-divider pt-10 kicker">Tagline</dt>
                  <dd class="m-0 flex flex-wrap items-baseline gap-x-10 gap-y-4 border-t-1 border-t-divider pt-10">
                    <span class="font-display text-1-15rem font-500 text-text-1 italic">{SITE_TAGLINE}</span>
                    <IdentityCopy label="text of the tagline" text="Copy text" value={SITE_TAGLINE} />
                  </dd>
                </dl>
              </div>
            )
          }}
        </IdentityPanel>
        <IdentityCaption>{{ default: () => sentence.value }}</IdentityCaption>
      </div>
    )
  }
})
