import { defineComponent } from 'vue'

import { FamilyChip } from '../../../kit'

export const RuleChrome = defineComponent({
  name: 'RuleChrome',
  setup() {
    return () => (
      <section aria-label="Rule family" class="mb-12 flex flex-wrap items-center gap-10">
        <FamilyChip />
      </section>
    )
  }
})
