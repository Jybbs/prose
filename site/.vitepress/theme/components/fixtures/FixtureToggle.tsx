import { mergeProps }                from '@zag-js/vue'
import { defineComponent, useModel } from 'vue'
import type { PropType }             from 'vue'

import { Tabs }           from '../../../kit'
import type { TabsScope } from '../../../kit'

import type { FixtureTab } from '../../../lib/shared/fixture-tab'

export const FixtureToggle = defineComponent({
  name  : 'FixtureToggle',
  props : {
    modelValue : { required: true, type: String as PropType<FixtureTab> }
  },
  emits : { 'update:modelValue': (value: FixtureTab) => value === 'before' || value === 'after' },
  setup(props) {
    const tab = useModel(props, 'modelValue')

    return () => (
      <Tabs label="Fixture view" modelValue={tab.value} onUpdate:modelValue={next => { tab.value = next }}>
        {{
          default: (tabs: TabsScope<string>) => (
            <div {...tabs.list} class="inline-flex rounded-sm border-1 border-divider bg-bg p-2">
              <button
                {...mergeProps(tabs.trigger('before'), { onClick: (event: MouseEvent) => event.stopPropagation() })}
                aria-controls={undefined}
                class="fixture-toggle-pill toggle-pill"
                data-side="before"
                type="button"
              >Before</button>
              <button
                {...mergeProps(tabs.trigger('after'), { onClick: (event: MouseEvent) => event.stopPropagation() })}
                aria-controls={undefined}
                class="fixture-toggle-pill toggle-pill"
                data-side="after"
                type="button"
              >After</button>
            </div>
          )
        }}
      </Tabs>
    )
  }
})
