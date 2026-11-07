import { useData }                   from 'vitepress'
import { computed, defineComponent } from 'vue'

import { FixtureCard } from '../fixtures/FixtureCard'

import { pageExamples } from '../../../lib/rules/rule-view'

export const RuleFixtures = defineComponent({
  name  : 'RuleFixtures',
  props : {
    rule : { required: true, type: String }
  },
  setup(props) {
    const { frontmatter } = useData()

    const examples = computed(() => pageExamples(frontmatter.value, props.rule))

    return () => examples.value.map(ex => <FixtureCard case={ex.case} key={ex.case} rule={props.rule} titleHtml={ex.titleHtml} />)
  }
})
