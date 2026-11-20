import { defineComponent, useModel } from 'vue'
import type { PropType }             from 'vue'

import { FixtureNoChange } from './FixtureNoChange'
import { FixtureToggle }   from './FixtureToggle'

import { FIXTURE_TABS, type FixtureTab } from '../../../lib/shared/fixture-tab'

// Holds a fixture card's Before and After toggle, or the No Change mark where
// `hasToggle` is unset, moving under the summary in an open card on a narrow
// screen and keeping a press inside it from reaching the card.
export const FixtureToggleSlot = defineComponent({
  name  : 'FixtureToggleSlot',
  props : {
    hasToggle  : Boolean,
    modelValue : { required: true, type: String as PropType<FixtureTab> }
  },
  emits : { 'update:modelValue': (value: FixtureTab) => FIXTURE_TABS.includes(value) },
  setup(props) {
    const tab = useModel(props, 'modelValue')

    return () => (
      <div
        class="max-mobile:group-data-open/fixture:col-start-2 max-mobile:group-data-open/fixture:row-2 max-mobile:group-data-open/fixture:-col-end-1 max-mobile:group-data-open/fixture:min-h-0 max-mobile:group-data-open/fixture:justify-start max-mobile:group-data-open/fixture:justify-self-start max-mobile:group-data-open/fixture:p-0 max-mobile:group-data-open/fixture:pb-8"
        onClick={event => event.stopPropagation()}
      >
        {props.hasToggle
          ? <FixtureToggle modelValue={tab.value} onUpdate:modelValue={next => { tab.value = next }} />
          : <FixtureNoChange />}
      </div>
    )
  }
})
