import { defineComponent } from 'vue'

import { NibBadge } from '../../../kit'

import { useHiddenTabindex } from '../../../lib/composables/use-aria-hidden'
import { data as rules }     from '../../../lib/rules/rules.data'
import { familyEmblem }      from '../../../lib/shared/emblems'
import { lookup }            from '../../../lib/shared/lookup'

export const InlineRuleLink = defineComponent({
  name  : 'InlineRuleLink',
  props : {
    slug : { required: true, type: String }
  },
  setup(props) {
    const tabindex = useHiddenTabindex()
    const rule     = lookup(rules.bySlug, props.slug, 'Inline rule link')

    return () => (
      <a
        class="rule-link"
        data-family={rule.family}
        data-rule={props.slug}
        href={rule.href}
        tabindex={tabindex.value}
      >
        <NibBadge emblem={familyEmblem(rule.family)} size={14} />
        {props.slug}
      </a>
    )
  }
})
