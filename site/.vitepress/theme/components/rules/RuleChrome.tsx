import { computed, defineComponent } from 'vue'

import { FamilyChip } from '../../../kit'

import { useCurrentRule }   from '../../../lib/composables/route'
import { data as pipeline } from '../../../lib/rules/pipeline.data'
import { formatFolio }      from '../../../lib/shared/numerals'
import { RuleOrderLine }    from './RuleOrderLine'

export const RuleChrome = defineComponent({
  name: 'RuleChrome',
  setup() {
    const rule     = useCurrentRule()
    const position = computed(() => pipeline.rules.find(entry => entry.slug === rule.value?.slug)?.position)

    return () => (
      <section aria-label="Rule family and order" class="mb-12 flex flex-wrap items-center gap-10">
        <FamilyChip />
        {position.value === undefined ? null : (
          <>
            <span class="folio">№ {formatFolio(position.value)}</span>
            <RuleOrderLine slug={rule.value!.slug} />
          </>
        )}
      </section>
    )
  }
})
