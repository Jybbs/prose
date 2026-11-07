import { useData }                   from 'vitepress'
import { computed, defineComponent } from 'vue'

import { CompositionCards } from './CompositionCards'
import { RuleCardList }     from './RuleCardList'
import { RuleConfigTable }  from './RuleConfigTable'
import { RuleFixtures }     from './RuleFixtures'
import { DocHeading }       from '../../../kit'
import { Fixture }          from '../fixtures/Fixture'

import { fixturesForRule, pageCompositionCases } from '../../../lib/rules/rule-view'

export const RuleLayout = defineComponent({
  name  : 'RuleLayout',
  props : {
    rule : { required: true, type: String }
  },
  setup(props, { slots }) {
    const { frontmatter } = useData()

    const canonical = fixturesForRule(props.rule).canonical
    const composes  = computed(() => pageCompositionCases(frontmatter.value, props.rule).length > 0)

    return () => (
      <>
        {slots.default?.()}
        <DocHeading id="configuration" title="Configuration" />
        {slots.configuration ? slots.configuration() : <RuleConfigTable />}
        {slots.facets ? (
          <>
            <DocHeading id="facets" title="Facets" />
            {slots.facets?.()}
          </>
        ) : null}
        <DocHeading id="the-canonical-case" title="The Canonical Case" />
        <Fixture case={canonical} rule={props.rule} />
        <DocHeading id="more-examples" title="More Examples" />
        <RuleFixtures rule={props.rule} />
        {composes.value ? <CompositionCards rule={props.rule} /> : null}
        <DocHeading id="related" title="Related" />
        <RuleCardList related />
        {slots['related-after']?.()}
      </>
    )
  }
})
