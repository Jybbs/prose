import { defineComponent } from 'vue'
import type { PropType }   from 'vue'

import { NibBadge } from '../../../kit'

import type { RuleSegment } from '../../../lib/rules/rule-view'
import { familyEmblem }     from '../../../lib/shared/emblems'

export const RuleSegmentChip = defineComponent({
  name  : 'RuleSegmentChip',
  props : {
    segment     : { required: true, type: Object as PropType<RuleSegment> },
    withTooltip : { required: true, type: Boolean }
  },
  setup(props) {
    return () => (
      <a
        class="composition-cards-chip box-border grid h-20 items-center justify-items-start bg-chip-ground font-mono leading-1-2 text-text-1 no-underline hover:text-brand-2"
        data-family={props.segment.family}
        data-rule={props.withTooltip ? props.segment.rule?.slug : undefined}
        href={props.segment.rule?.href}
        title={props.segment.rule ? undefined : `${props.segment.slug} (${props.segment.family ?? 'undocumented'})`}
      >
        {props.segment.family && (
          <NibBadge
            class="block justify-self-center text-family-nib-chip group-data-[edits=false]/fixture:text-text-3-chip-31"
            emblem={familyEmblem(props.segment.family)}
            size={13}
          />
        )}
        <span class="composition-cards-chip-label z-4 block min-w-0 whitespace-nowrap">
          <span class="composition-cards-chip-slug text-xxs tracking-0-04em uppercase">{props.segment.slug}</span>
        </span>
      </a>
    )
  }
})
