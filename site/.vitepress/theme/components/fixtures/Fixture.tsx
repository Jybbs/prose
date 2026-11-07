import { useData }                        from 'vitepress'
import { computed, defineComponent, ref } from 'vue'

import { FixtureConfig }  from './FixtureConfig'
import { FixtureLead }    from './FixtureLead'
import { FixturePairDoc } from './FixturePairDoc'
import { FixtureToggle }  from './FixtureToggle'
import { SettingGauge }   from './SettingGauge'

import { useFixtureEntry } from '../../../lib/composables/use-fixture-entry'
import { fixtureGauge }    from '../../../lib/fixtures/entry'
import type { FixtureTab } from '../../../lib/shared/fixture-tab'

export const Fixture = defineComponent({
  name  : 'Fixture',
  props : {
    case    : { required: true, type: String },
    rule    : { required: true, type: String },
    setting : String
  },
  setup(props) {
    const { frontmatter } = useData()
    const entry           = useFixtureEntry(props)
    const activeTab       = ref<FixtureTab>('after')

    // A `setting` steps the case through that setting's values on a gauge, the
    // output following the value the gauge holds.
    const gauge = computed(() => props.setting
      ? fixtureGauge(frontmatter.value, props.rule, props.case, props.setting)
      : null)
    const value      = ref(gauge.value?.default ?? 0)
    const outputHtml = computed(() => gauge.value
      ? gauge.value.outputs[gauge.value.steps[value.value]!]!
      : entry.value.outputHtml)

    return () => (
      <div class="fixture my-1-25rem">
        <FixtureLead nodes={entry.value.descriptionNodes} />
        {entry.value.hasToggle ? (
          <header class="fixture-bar flex justify-end pb-8">
            <FixtureToggle modelValue={activeTab.value} onUpdate:modelValue={next => { activeTab.value = next }} />
          </header>
        ) : null}
        {entry.value.configHtml ? <FixtureConfig html={entry.value.configHtml} /> : null}
        {gauge.value ? <SettingGauge gauge={gauge.value} modelValue={value.value} onUpdate:modelValue={next => { value.value = next }} /> : null}
        <FixturePairDoc
          activeTab={activeTab.value}
          inputHtml={entry.value.inputHtml}
          outputHtml={outputHtml.value}
          sandboxSeed={entry.value.sandboxSeed}
        />
      </div>
    )
  }
})
