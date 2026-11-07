import {
  clamp,
  useElementSize,
  useMouseInElement,
  useRafFn,
  useScroll,
  useTimeoutFn
} from '@vueuse/core'
import { computed, defineComponent, ref, shallowRef, watch } from 'vue'
import type { PropType }                                     from 'vue'

import { useHiddenTabindex } from '../../../../lib/composables/use-aria-hidden'
import { useReducedMotion }  from '../../../../lib/composables/use-reduced-motion'
import type { RenderedRule } from '../../../../lib/rules/rules.data'
import { MS_PER_SEC }        from '../../../../lib/shared/constants'
import { SurfacePip }        from './SurfacePip'
import { SurfaceRailName }   from './SurfaceRailName'

const EDGE_PX      = 34

const GLIDE_PX_SEC = 260

const JUMP_MS      = 420

const PINNED_MIN   = 3

const SETTLE_SEC   = 0.09

const SLACK_PX     = 1

const SLOT_PX      = 24

const RAIL_END_CLASS = 'surface-rail-end before:pointer-events-none before:absolute before:inset-0 before:z-0 before:m-auto before:rounded-pill before:border-1 before:border-family/30 before:transition-colors before:duration-240 before:ease-linear hover:before:border-family/64 data-active:before:border-family/64'

export const SurfaceRail = defineComponent({
  name  : 'SurfaceRail',
  props : {
    rules : { required: true, type: Array as PropType<readonly RenderedRule[]> }
  },
  setup(props) {
    const railRef = shallowRef<HTMLElement | null>(null)
    const windowRef = shallowRef<HTMLElement | null>(null)
    const reducedMotion = useReducedMotion()

    const { width: railWidth }    = useElementSize(railRef)
    const { width: windowWidth }  = useElementSize(windowRef)
    const { elementX, isOutside } = useMouseInElement(windowRef)
    const { x: scrolled }         = useScroll(windowRef)

    const heading    = ref(1)
    const hoveredIdx = ref<number | null>(null)
    const jumping    = ref(false)

    const activeIdx  = computed(() => hoveredIdx.value ?? 0)
    const activeRule = computed(() => props.rules[activeIdx.value])
    const hovered    = computed(() => hoveredIdx.value !== null)
    const tabindex   = useHiddenTabindex()

    const overflows = computed(() => railWidth.value > 0 && SLOT_PX * props.rules.length > railWidth.value)
    const pinned    = computed(() => overflows.value && props.rules.length > PINNED_MIN)

    const inner = computed(() => {
      const indices = props.rules.map((_, index) => index)
      return pinned.value ? indices.slice(1, -1) : indices
    })

    const travel  = computed(() => Math.max(0, SLOT_PX * inner.value.length - windowWidth.value))
    const travels = computed(() => travel.value > SLACK_PX)

    const atStart = computed(() => scrolled.value <= 0.5)
    const atEnd   = computed(() => scrolled.value >= travel.value - 0.5)
    const behind  = computed(() => (travels.value ? clamp(scrolled.value / travel.value, 0, 1) : 0))

    const chevrons = computed(() => (travels.value ? [
      { at: atStart.value, glyph: '‹', label: 'Travel to the first rule', reach: behind.value,     side: 'start', toEnd: false },
      { at: atEnd.value,   glyph: '›', label: 'Travel to the last rule',  reach: 1 - behind.value, side: 'end',   toEnd: true  }
    ] : []))

    const edge = computed(() => {
      if (isOutside.value || jumping.value || !travels.value) return 'none'
      if (elementX.value < EDGE_PX && !atStart.value) return 'start'
      if (windowWidth.value - elementX.value < EDGE_PX && !atEnd.value) return 'end'
      return 'none'
    })

    const depth = computed(() => {
      if (edge.value === 'start') return 1 - Math.max(0, elementX.value) / EDGE_PX
      if (edge.value === 'end')   return 1 - Math.max(0, windowWidth.value - elementX.value) / EDGE_PX
      return 0
    })

    const velocity = computed(() => {
      if (edge.value === 'none') return 0
      return (edge.value === 'start' ? -GLIDE_PX_SEC : GLIDE_PX_SEC) * depth.value ** 2
    })

    let applied = 0

    function pipBinding(index: number) {
      return {
        active   : hovered.value && index === activeIdx.value,
        distance : hovered.value ? Math.abs(index - activeIdx.value) : 0,
        index,
        rule     : props.rules[index]
      }
    }

    function select(index: number): void {
      heading.value    = index < activeIdx.value ? -1 : 1
      hoveredIdx.value = index
    }

    function selectUnderPointer(): void {
      if (isOutside.value) return
      const index = inner.value[Math.min(
        inner.value.length - 1,
        Math.max(0, Math.floor(((windowRef.value?.scrollLeft ?? 0) + elementX.value) / SLOT_PX))
      )]
      if (index !== undefined && index !== activeIdx.value) select(index)
    }

    const { pause, resume } = useRafFn(({ delta }) => {
      const el = windowRef.value
      if (!el) return
      const step = delta / MS_PER_SEC
      applied = reducedMotion.value
        ? velocity.value
        : applied + (velocity.value - applied) * (1 - Math.exp(-step / SETTLE_SEC))
      if (velocity.value === 0 && Math.abs(applied) < 1) {
        applied = 0
        pause()
        return
      }
      el.scrollLeft += applied * step
      selectUnderPointer()
    }, { immediate: false })

    const { start: releaseJump } = useTimeoutFn(() => { jumping.value = false }, JUMP_MS, { immediate: false })

    watch(velocity, moving => { if (moving !== 0) resume() }, { immediate: true })

    function jump(toEnd: boolean): void {
      const el = windowRef.value
      if (!el) return
      jumping.value = true
      applied       = 0
      el.scrollTo({
        behavior : reducedMotion.value ? 'auto' : 'smooth',
        left     : toEnd ? el.scrollWidth : 0
      })
      releaseJump()
    }

    const swap = computed(() => (heading.value < 0 ? 'surface-rail-back' : 'surface-rail-fwd'))

    const railStyle = computed(() => ({
      '--rail-inset' : pinned.value ? `${SLOT_PX}px` : '0px',
      '--slot'       : `${SLOT_PX}px`
    }))

    const windowStyle = computed(() => ({
      '--fade-end'   : atEnd.value   ? '0px' : '20px',
      '--fade-start' : atStart.value ? '0px' : '20px'
    }))

    return () => (
      <div class="surface-rail relative mt-auto flex flex-col gap-10 pt-18 before:absolute before:inset-x-0 before:top-0 before:h-1 before:bg-divider before:opacity-65">
        <div
          class="surface-rail-row pointer-events-auto relative z-2 flex items-center"
          data-bookends={pinned.value}
          ref={railRef}
          style={railStyle.value}
        >
          {pinned.value && (
            <SurfacePip
              {...pipBinding(0)}
              class={RAIL_END_CLASS}
              onSelect={() => select(0)}
            />
          )}
          <div
            class="surface-rail-window relative min-w-0 flex-auto scrollbar-none overflow-x-auto overflow-y-hidden overscroll-x-contain transition-mask-image duration-260 ease-linear data-[edge=end]:cursor-e-resize data-[edge=start]:cursor-w-resize"
            data-edge={edge.value}
            ref={windowRef}
            style={windowStyle.value}
          >
            <div class="surface-rail-track flex w-max items-center">
              {inner.value.map(index => <SurfacePip key={props.rules[index].slug} {...pipBinding(index)} onSelect={() => select(index)} />)}
            </div>
          </div>
          {chevrons.value.map(chevron => (
            <button
              aria-label={chevron.label}
              class="surface-rail-chevron absolute top-1/2 z-3 grid w-18 -translate-y-1/2 cursor-pointer place-items-center border-0 font-mono text-15 leading-none text-family hover:opacity-100 focus-visible:opacity-100 focus-visible:outline-offset-1 active:scale-86 disabled:pointer-events-none disabled:opacity-0 data-[side=end]:hover:translate-x-3 data-[side=start]:hover:-translate-x-3"
              data-side={chevron.side}
              disabled={chevron.at}
              key={chevron.side}
              onClick={() => jump(chevron.toEnd)}
              style={{ '--reach': chevron.reach }}
              tabindex={tabindex.value}
              type="button"
            >{chevron.glyph}</button>
          ))}
          {pinned.value && (
            <SurfacePip
              {...pipBinding(props.rules.length - 1)}
              class={RAIL_END_CLASS}
              onSelect={() => select(props.rules.length - 1)}
            />
          )}
        </div>
        <div aria-live="polite" class="pointer-events-auto relative z-2 min-h-26">
          <SurfaceRailName rule={activeRule.value} swap={swap.value} />
        </div>
      </div>
    )
  }
})
