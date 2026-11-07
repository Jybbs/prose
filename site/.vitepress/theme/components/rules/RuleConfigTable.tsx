import { computed, defineComponent } from 'vue'

import { data as configs } from '../../../lib/rules/rule-configs.data'
import { useCurrentRule }  from '../../../lib/composables/route'
import { ConfigRowTable }  from '../base/ConfigRowTable'

export const RuleConfigTable = defineComponent({
  name: 'RuleConfigTable',
  setup() {
    const current = useCurrentRule()

    const rows = computed(() => configs[current.value?.slug ?? ''] ?? [])

    return () => <ConfigRowTable rows={rows.value} />
  }
})
