import { computed, defineComponent } from 'vue'
import type { PropType }             from 'vue'

import { data as configKeys, type ConfigKeys } from '../../../lib/reference/config-keys.data'
import { data as ruleConfigs }                 from '../../../lib/rules/rule-configs.data'

export const ConfigDefault = defineComponent({
  name  : 'ConfigDefault',
  props : {
    facet   : { required: true, type: String },
    rule    : { default: undefined, type: String },
    section : { default: 'top', type: null as unknown as PropType<keyof ConfigKeys> }
  },
  setup(props) {
    const value = computed(() => {
      const rows = props.rule === undefined ? configKeys[props.section] : ruleConfigs[props.rule]
      const row  = rows?.find(r => r.key === props.facet)
      if (row === undefined) {
        throw new Error(`ConfigDefault: no key ${props.facet} under ${props.rule ?? props.section}`)
      }
      return row.default
    })

    return () => <code>{value.value}</code>
  }
})
