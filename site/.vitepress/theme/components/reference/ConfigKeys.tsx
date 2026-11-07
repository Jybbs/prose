import { computed, defineComponent } from 'vue'
import type { PropType }             from 'vue'

import { data, type ConfigKeys as Keys } from '../../../lib/reference/config-keys.data'
import { ConfigRowTable }                from '../base/ConfigRowTable'

export const ConfigKeys = defineComponent({
  name  : 'ConfigKeys',
  props : {
    section : { required: true, type: null as unknown as PropType<keyof Keys> }
  },
  setup(props) {
    const rows = computed(() => data[props.section])

    return () => <ConfigRowTable rows={rows.value} />
  }
})
