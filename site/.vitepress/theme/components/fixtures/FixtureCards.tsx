import { computed, defineComponent } from 'vue'
import type { PropType }             from 'vue'

import { FixtureCard } from './FixtureCard'

import { caseNames } from '../../../lib/fixtures/entry'

export const FixtureCards = defineComponent({
  name  : 'FixtureCards',
  props : {
    cases : { required: true, type: [Array, String] as PropType<readonly string[] | string> },
    rule  : { required: true, type: String }
  },
  setup(props) {
    const names = computed(() => typeof props.cases === 'string' ? caseNames(props.cases) : props.cases)

    return () => names.value.map(name => <FixtureCard case={name} key={name} rule={props.rule} />)
  }
})
