import { computed, defineComponent, Fragment, shallowRef } from 'vue'

import { useCarouselMeasurement } from '../../../../lib/composables/use-carousel-measurement'
import { useCarouselVelocity }    from '../../../../lib/composables/use-carousel-velocity'
import { useReducedMotion }       from '../../../../lib/composables/use-reduced-motion'
import { data as landing }        from '../../../../lib/landing/landing.data'
import { data as rules }          from '../../../../lib/rules/rules.data'
import { LandingSection }         from '../LandingSection'
import { SurfaceCardBase }        from './SurfaceCardBase'

const BASE_SPEED_PX_PER_SEC = 32

const EDGE_MARGIN_PX        = 32

const MAGNET_GAIN           = 4

const MAX_PULL_PX_PER_SEC   = BASE_SPEED_PX_PER_SEC * 8

export const Surfaces = defineComponent({
  name: 'Surfaces',
  setup() {
    const surfaceCards = computed(() =>
      landing.surfaces.map(s => ({ ...s, rules: rules.byFamily[s.family] ?? [] }))
    )

    const ruleCount = computed(() => rules.list.length)

    const heading = computed(() =>
      `<strong>${surfaceCards.value.length}</strong> rule families. <em>${ruleCount.value}</em> rules.`
    )

    const trackRef = shallowRef<HTMLElement | null>(null)
    const viewportRef = shallowRef<HTMLElement | null>(null)
    const reducedMotion = useReducedMotion()

    const { fits, halfWidth } = useCarouselMeasurement(trackRef, viewportRef, () => surfaceCards.value.length)
    const { offset, onPointerLeave, onPointerMove } = useCarouselVelocity(viewportRef, halfWidth, fits, {
      baseSpeedPxPerSec : BASE_SPEED_PX_PER_SEC,
      edgeMarginPx      : EDGE_MARGIN_PX,
      magnetGain        : MAGNET_GAIN,
      maxPullPxPerSec   : MAX_PULL_PX_PER_SEC,
      reducedMotion
    })

    const trackStyle = computed(() => ({
      transform: `translate3d(${-offset.value}px, 0, 0)`
    }))

    return () => (
      <LandingSection centered heading={heading.value} kicker="The Surfaces">
        {{
          default: () => (
            <div
              aria-label="Rule family carousel"
              class="surfaces-carousel group/carousel relative mt-8 w-screen overflow-hidden pt-32 pb-8 mask-x-from-95% data-fits:mask-none"
              data-fits={fits.value || undefined}
              onPointerleave={onPointerLeave}
              onPointermove={onPointerMove}
              ref={viewportRef}
            >
              <div
                class="flex w-max items-stretch gap-18 px-32 will-change-transform *:flex-none *:basis-360 group-data-fits/carousel:w-full group-data-fits/carousel:justify-center max-desktop:gap-14 max-desktop:*:basis-300 max-mobile:gap-12 max-mobile:*:basis-260"
                ref={trackRef}
                style={trackStyle.value}
              >
                {Array.from({ length: 2 }, (_, index) => index + 1).map(copy => (
                  <Fragment key={copy}>
                    {surfaceCards.value.map(card => (
                      <SurfaceCardBase
                        bodyNodes={card.bodyNodes}
                        class={fits.value && copy === 2 ? 'hidden' : 'flex'}
                        duplicate={copy === 2}
                        family={card.family}
                        key={`${copy}-${card.family}`}
                        number={card.number}
                        rules={card.rules}
                      />
                    ))}
                  </Fragment>
                ))}
              </div>
            </div>
          ),
          'heading-extra': () => (
            <a
              class="mt-18 inline-flex items-center rounded-sm border-1 border-divider bg-bg-soft px-12 py-4 kicker text-text-2 transition-colors hover:border-brand-1 hover:bg-bg hover:text-brand-1 focus-visible:border-brand-1 focus-visible:bg-bg focus-visible:text-brand-1"
              href="/rules/"
            >All rules</a>
          )
        }}
      </LandingSection>
    )
  }
})
