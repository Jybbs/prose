import { useData }                   from 'vitepress'
import { computed, defineComponent } from 'vue'

import { FixtureCards } from '../fixtures/FixtureCards'

import { pageExamples } from '../../../lib/rules/rule-view'

export const RuleFixtures = defineComponent({
  name  : 'RuleFixtures',
  props : {
    rule : { required: true, type: String }
  },
  setup(props) {
    const { frontmatter } = useData()

    const examples = computed(() => pageExamples(frontmatter.value, props.rule))

    return () => <FixtureCards cases={examples.value} rule={props.rule} />
  }
})
