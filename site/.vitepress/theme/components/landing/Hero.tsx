import { defaultDocument, useElementBounding }             from '@vueuse/core'
import { computed, defineComponent, Fragment, shallowRef } from 'vue'

import { tileStamps }                from '../../../lib/landing/hero-stamps'
import { heroGrid, watermarkHeight } from '../../../lib/landing/hero-tiling'
import { SITE_ALT }                  from '../../../lib/shared/constants'

export const Hero = defineComponent({
  name: 'Hero',
  setup() {
    const heroRef  = shallowRef<HTMLElement | null>(null)
    const layerRef = shallowRef<HTMLElement | null>(null)

    const { top:   heroTop }     = useElementBounding(heroRef)
    const { width: layerWidth }  = useElementBounding(layerRef)
    const { top:   terminusTop } = useElementBounding(() =>
      defaultDocument?.querySelector<HTMLElement>('.surfaces-carousel') ?? null
    )

    const layerHeight = computed(() => {
      const height = watermarkHeight(heroTop.value, terminusTop.value)
      return height > 0 ? height : null
    })

    const layerStyle = computed(() =>
      layerHeight.value !== null ? { height: `${layerHeight.value}px`, minHeight: '0' } : undefined
    )

    const grid   = computed(() => heroGrid(layerHeight.value ?? 0, layerWidth.value))
    const stamps = computed(() => tileStamps(grid.value.cols, grid.value.rows))

    return () => (
      <div
        class="relative isolate flex items-center justify-center px-24 pt-88 pb-32"
        ref={heroRef}
      >
        <div
          aria-hidden="true"
          class="pointer-events-none absolute -top-24 left-1/2 z-0 min-h-watermarks w-screen -translate-x-1/2 overflow-clip mask-b-from-80%"
          ref={layerRef}
          style={layerStyle.value}
        >
          {stamps.value.map((s, i) => (
            <Fragment key={i}>
              {s.kind === 'big'
                ? (
                  <img
                    alt=""
                    class="landing-hero-watermark absolute w-110 text-ube opacity-8 will-change-transform select-none max-card-stack:w-80"
                    src="/logo.svg"
                    style={{
                                '--rotation' : `${s.rotate}deg`,
                                left         : `${s.x}%`,
                                top          : `${s.y}px`
                              }}
                  />
                )
                : (
                <span
                  class="landing-hero-watermark absolute font-display text-2-4rem leading-none font-700 text-ube opacity-8 will-change-transform select-none max-card-stack:text-1-6rem"
                  style={{
                              '--rotation' : `${s.rotate}deg`,
                              left         : `${s.x}%`,
                              top          : `${s.y}px`
                            }}
                >{s.letter}</span>
              )}
            </Fragment>
          ))}
        </div>
        <h1 class="relative z-1 w-wordmark">
          <img alt={SITE_ALT} class="w-full" src="/title-with-tagline.svg" />
        </h1>
      </div>
    )
  }
})
