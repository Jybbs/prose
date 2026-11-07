import { computed, defineComponent } from 'vue'
import type { PropType }             from 'vue'

import { data, type FlagTable } from '../../../lib/reference/cli-flags.data'
import { ConfigRowTable }       from '../base/ConfigRowTable'

export const CliFlags = defineComponent({
  name  : 'CliFlags',
  props : {
    command : { required: true, type: String as PropType<FlagTable> }
  },
  setup(props) {
    const rows = computed(() => data[props.command])

    return () => <ConfigRowTable head="Flag" rows={rows.value} />
  }
})
