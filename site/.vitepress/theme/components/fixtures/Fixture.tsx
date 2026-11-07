import { defineComponent, ref } from 'vue'

import { FixturePairDoc } from './FixturePairDoc'
import { FixtureToggle }  from './FixtureToggle'

import { useFixtureEntry } from '../../../lib/composables/use-fixture-entry'
import type { FixtureTab } from '../../../lib/shared/fixture-tab'
import { InlineProse }     from '../base/InlineProse'

export const Fixture = defineComponent({
  name  : 'Fixture',
  props : {
    case : { required: true, type: String },
    rule : { required: true, type: String }
  },
  setup(props) {
    const entry     = useFixtureEntry(props)
    const activeTab = ref<FixtureTab>('after')

    return () => (
      <div class="fixture my-1-25rem">
        {entry.value.descriptionNodes && (
          <div class="fixture-lead mb-1rem">
            <InlineProse nodes={entry.value.descriptionNodes} />
          </div>
        )}
        {entry.value.hasToggle && (
          <header class="fixture-bar flex justify-end pb-8">
            <FixtureToggle modelValue={activeTab.value} onUpdate:modelValue={next => { activeTab.value = next }} />
          </header>
        )}
        <FixturePairDoc
          activeTab={activeTab.value}
          inputHtml={entry.value.inputHtml}
          outputHtml={entry.value.outputHtml}
        />
      </div>
    )
  }
})
