import { computed, defineComponent, ref } from 'vue'
import type { PropType }                  from 'vue'

import { portsOf }            from '../../../lib/architecture/ports'
import { data as primitives } from '../../../lib/architecture/primitives-composition.data'

import { pickOr }                   from '../../../lib/shared/pick-or'
import { PRIMITIVE_LAYER_NUMERALS } from '../../../lib/shared/registries'
import type { PrimitiveSlug }       from '../../../lib/shared/registries'

import { PrimitivesCompositionDetail } from './PrimitivesCompositionDetail'
import { PrimitivesCompositionGrid }   from './PrimitivesCompositionGrid'

export const PrimitivesComposition = defineComponent({
  name  : 'PrimitivesComposition',
  props : {
    initialFocus : null as unknown as PropType<PrimitiveSlug>
  },
  setup(props) {
    const focused = ref<PrimitiveSlug | null>(props.initialFocus ?? null)

    const bands = computed(() => [
      { entries : primitives.byLayer.analysis,      key : 'analysis'      as const, numeral : PRIMITIVE_LAYER_NUMERALS.analysis      },
      { entries : primitives.byLayer.orchestration, key : 'orchestration' as const, numeral : PRIMITIVE_LAYER_NUMERALS.orchestration },
      { entries : primitives.byLayer.base,          key : 'base'          as const, numeral : PRIMITIVE_LAYER_NUMERALS.base          }
    ])

    const focusedEntry = computed(() =>
      focused.value === null ? null : pickOr(primitives.bySlug, focused.value, null))

    const ports = computed(() => portsOf(focusedEntry.value))

    function setFocus(slug: PrimitiveSlug) {
      focused.value = slug
    }

    function clearFocus() {
      if (props.initialFocus !== undefined) focused.value = props.initialFocus
    }

    return () => (
      <div
        class="primitives-composition my-1-5rem panel bg-bg px-14 py-12"
        data-layer={focusedEntry.value?.layer ?? 'empty'}
      >
        <PrimitivesCompositionGrid
          bands={bands.value}
          focused={focused.value}
          onBlur={clearFocus}
          onFocus={setFocus}
          ports={ports.value}
        />
        {props.initialFocus === undefined && <PrimitivesCompositionDetail focused={focused.value} />}
      </div>
    )
  }
})
