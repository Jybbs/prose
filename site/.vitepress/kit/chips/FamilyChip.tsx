import { computed, defineComponent } from 'vue'
import type { PropType }             from 'vue'

import { useCurrentRule } from '../../lib/composables/route'
import { familyEmblem }   from '../../lib/shared/emblems'
import * as registries    from '../../lib/shared/registries'

import { NibBadge } from '../drawing/NibBadge'

import { Chip } from './Chip'

// Renders a family's chip, reading the current rule's family where `family`
// is unset, linked to the family's page unless `linked` is false.
export const FamilyChip = defineComponent({
  name  : 'FamilyChip',
  props : {
    family : String as PropType<registries.GlossaryFamily>,
    linked : { default: true, type: Boolean }
  },
  setup(props) {
    const rule   = useCurrentRule()
    const family = computed(() => props.family ?? rule.value?.family ?? null)
    const emblem = computed(() => family.value && familyEmblem(family.value))
    const href   = computed(() => props.linked && family.value && family.value in registries.FAMILY_META
      ? `/rules/${family.value}/`
      : undefined)

    return () => {
      const current = emblem.value
      return current
        ? (
          <Chip class="nib-ground" family={current.family} href={href.value} variant="family-chip">
            {{
              default: () => [
                <NibBadge emblem={current} size={15.2} />,
                <span>{current.name}</span>
              ]
            }}
          </Chip>
        )
        : null
    }
  }
})
