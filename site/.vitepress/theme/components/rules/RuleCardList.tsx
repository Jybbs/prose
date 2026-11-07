import { computed, defineComponent } from 'vue'
import type { PropType }             from 'vue'

import { RuleCard } from './RuleCard'

import { data as rules }   from '../../../lib/rules/rules.data'
import { useCurrentRule }  from '../../../lib/composables/route'
import { lookup }          from '../../../lib/shared/lookup'
import type { RuleFamily } from '../../../lib/shared/registries'

export const RuleCardList = defineComponent({
  name  : 'RuleCardList',
  props : {
    family  : null as unknown as PropType<RuleFamily>,
    related : Boolean as PropType<true>
  },
  setup(props) {
    const current = useCurrentRule()

    const items = computed(() => {
      if (props.related) {
        return current.value?.related.map(s => lookup(rules.bySlug, s, 'Related rule')) ?? []
      }
      return rules.list.filter(r => !props.family || r.family === props.family)
    })

    return () => (
      items.value.length > 0
        ? (
          <div class={['flex max-w-960 flex-col gap-6', props.related ? 'mt-28' : 'mt-24']}>
            {!props.related && <h2 class="sr-only">Rules</h2>}
            {items.value.map(rule => <RuleCard key={rule.slug} rule={rule} />)}
          </div>
        )
        : !props.related
        ? <p class="text-sm text-text-3">No rules in this group yet.</p>
        : null
    )
  }
})
