import { clamp, defaultWindow, useEventListener }                        from '@vueuse/core'
import { computed, defineComponent, onMounted, ref, useId, watchEffect } from 'vue'

import { Slider }             from '../../../kit'
import type { SliderScope }   from '../../../kit'
import { data }               from '../../../lib/identity/identity.data'
import { fileDrawing, PARTS } from '../../../lib/identity/marks'
import { IdentityCopyChip }   from './IdentityParts'

import type { Drawing, PartId } from '../../../lib/identity/marks'

const LOCKUP = data.marks.files['title-with-tagline.svg']

// Writes a drawing as an `<svg>` set at `height` pixels, its width following
// its box.
const svgOf = (drawing: Drawing, height: number): string =>
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${drawing.viewBox}" height="${height}" aria-hidden="true">${drawing.markup}</svg>`

// Composes the pen-nib P, the letters, and the tagline the switches turn on
// into a copy of the site's nav title, whose own name hides while the letters
// draw it, naming the file that draws those parts, and resizes the composed
// mark from a handle at its corner or a ruler beneath it that marks the height
// the nav bar serves the mark at.
export const IdentityMarks = defineComponent({
  name : 'IdentityMarks',
  setup() {
    const on       = ref<PartId[]>(PARTS.map(part => part.id))
    const height   = ref(0)
    const served   = ref(0)
    const range    = ref({ max: 0, min: 0 })
    const stage    = ref<HTMLElement | null>(null)
    const slot     = ref<HTMLElement | null>(null)
    const wordmark = ref<HTMLElement | null>(null)
    const corner   = ref({ left: 0, top: 0 })
    const trackId  = useId()
    const file     = computed(() => fileDrawing(on.value))
    const drawing  = computed((): Drawing => file.value
      ? data.marks.files[file.value]
      : { markup: on.value.map(id => data.marks.parts[id].markup).join(''), viewBox: LOCKUP.viewBox })
    const percent  = (px: number): number => (px - range.value.min) / (range.value.max - range.value.min) * 100

    const toggle = (id: PartId): void => {
      on.value = on.value.includes(id) ? on.value.filter(each => each !== id) : PARTS.map(part => part.id).filter(each => each === id || on.value.includes(each))
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

    useEventListener(defaultWindow, 'resize', measure)

    onMounted(() => {
      const title = document.querySelector<HTMLElement>('.VPNavBarTitle')
      const logo  = title?.querySelector<HTMLElement>('.logo')
      if (!title || !logo || !measure()) return
      const copy   = title.cloneNode(true) as HTMLElement
      const holder = document.createElement('span')
      for (const { name, value } of logo.attributes) if (name.startsWith('data-v-') || name === 'class') holder.setAttribute(name, value)
      copy.querySelector('.logo')!.replaceWith(holder)
      stage.value!.append(copy)
      wordmark.value = holder.nextElementSibling as HTMLElement | null
      slot.value = holder
    })

    watchEffect(() => {
      if (!slot.value) return
      slot.value.innerHTML = on.value.length > 0 ? svgOf(drawing.value, Math.round(height.value)) : ''
      wordmark.value?.toggleAttribute('hidden', on.value.includes('letters'))
      const mark = slot.value.getBoundingClientRect()
      const box  = stage.value!.getBoundingClientRect()
      corner.value = { left: mark.right - box.left, top: mark.bottom - box.top }
    })

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
      const box = (event.currentTarget as HTMLElement).getBoundingClientRect()
      drag(event, next => range.value.min + (next.clientX - box.left) / box.width * (range.value.max - range.value.min))
    }

    return () => (
      <div class="my-1-5rem grid gap-1-25rem">
        <div class="flex flex-wrap gap-x-1-5rem gap-y-0-75rem">
          {PARTS.map(part => (
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
        <div class="relative">
          <div class="identity-nav flex items-center overflow-hidden rounded-sm border-1 border-divider bg-bg px-1-25rem" inert ref={stage} />
          {served.value > 0 && on.value.length > 0 && (
            <span
              aria-hidden="true"
              class="absolute size-14 -translate-1/2 cursor-nwse-resize touch-none rounded-sm border-1 border-text-2 bg-bg"
              onPointerdown={fromHandle}
              style={{ left: `${corner.value.left}px`, top: `${corner.value.top}px` }}
            />
          )}
        </div>
        <p aria-live="polite" class="m-0 flex flex-wrap items-center gap-8 text-sm text-text-2">
          {file.value
            ? [<span key="lead">Drawn by</span>, <IdentityCopyChip key="file" label="the file path" value={`/${file.value}`} />]
            : on.value.length > 0
            ? 'No served file draws these parts alone, so they sit in the lockup’s own box.'
            : 'Turn a part on to compose a mark.'}
        </p>
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
            <p class="m-0 flex justify-between font-mono text-meta text-text-3 tabular-nums">
              <span>{`${Math.round(range.value.min)}px`}</span>
              <span>{`${Math.round(height.value)}px · served at ${Math.round(served.value)}px`}</span>
              <span>{`${Math.round(range.value.max)}px`}</span>
            </p>
          </div>
        )}
      </div>
    )
  }
})
