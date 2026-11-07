import { useElementHover }                       from '@vueuse/core'
import { computed, defineComponent, shallowRef } from 'vue'
import type { PropType }                         from 'vue'

import { provideAriaHidden } from '../../../../lib/composables/use-aria-hidden'
import type { InlineNode }   from '../../../../lib/markdown/inline-nodes'
import type { RenderedRule } from '../../../../lib/rules/rules.data'
import { familyEmblem }      from '../../../../lib/shared/emblems'
import * as registries       from '../../../../lib/shared/registries'
import { InlineProse }       from '../../base/InlineProse'
import { NibBadge }          from '../../../../kit'
import { SurfaceRail }       from './SurfaceRail'

export const SurfaceCardBase = defineComponent({
  name  : 'SurfaceCardBase',
  props : {
    bodyNodes : { required: true, type: Array as PropType<InlineNode[]> },
    duplicate : { required: true, type: Boolean },
    family    : { required: true, type: null as unknown as PropType<registries.RuleFamily> },
    number    : { required: true, type: String },
    rules     : { required: true, type: Array as PropType<readonly RenderedRule[]> }
  },
  setup(props) {
    provideAriaHidden(() => props.duplicate)

    const href     = computed(() => `/rules/${props.family}/`)
    const meta     = computed(() => registries.FAMILY_META[props.family])
    const tabindex = computed(() => (props.duplicate ? -1 : undefined))

    const rootRef = shallowRef<HTMLElement | null>(null)

    const active = useElementHover(rootRef)

    return () => (
      <div
        aria-hidden={props.duplicate || undefined}
        class="surface-card relative isolate min-h-320 min-w-0 flex-col panel bg-bg-soft p-28 text-text-1 transition-colors duration-slow focus-within:border-family-edge-soft data-[active=true]:border-family-edge-soft motion-reduce:transition-none"
        data-active={active.value}
        data-family={props.family}
        ref={rootRef}
      >
        <a
          aria-label={`See all ${meta.value.label.toLowerCase()} rules`}
          class="surface-card-cover-link"
          href={href.value}
          tabindex={tabindex.value}
        />
        <span class="pointer-events-none relative z-1 font-mono text-label tracking-label text-text-3">— {props.number}</span>
        <span class="surface-card-icon pointer-events-none absolute top-22 right-22 z-2 flex size-56 items-center justify-center">
          <NibBadge emblem={familyEmblem(props.family)} size={36} />
        </span>
        <h3 class="pointer-events-none relative z-1 mt-42 mb-12 border-none p-0 font-display text-2rem leading-1-05 font-500 text-balance text-text-1">{meta.value.label}</h3>
        <p class="surface-card-blurb pointer-events-none relative z-1 m-0 font-base text-md leading-1-6 text-text-2 in-code:font-mono in-code:text-0-9em">
          <InlineProse nodes={props.bodyNodes} />
        </p>
        <SurfaceRail class="pointer-events-none z-1" rules={props.rules} />
      </div>
    )
  }
})
