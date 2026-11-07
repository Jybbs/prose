import { computed, defineComponent } from 'vue'
import type { PropType }             from 'vue'

import { data as pipeline } from '../../../lib/rules/pipeline.data'
import { InlineRuleLink }   from './InlineRuleLink'

export const TreeRules = defineComponent({
  name  : 'TreeRules',
  props : {
    tree : { required: true, type: String as PropType<'changed' | 'kept'> }
  },
  setup(props) {
    const rules = computed(() =>
      pipeline.rules.filter(rule => rule.preservesTree === (props.tree === 'kept')))

    return () => (
      <ul>
        {rules.value.map(rule => (
          <li key={rule.slug}>
            {rule.documented
              ? <InlineRuleLink slug={rule.slug} />
              : <code>{rule.slug}</code>}
          </li>
        ))}
      </ul>
    )
  }
})
