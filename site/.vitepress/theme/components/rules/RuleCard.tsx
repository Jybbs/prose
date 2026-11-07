import { defineComponent, ref } from 'vue'
import type { PropType }        from 'vue'

import { MiddleEllipsis, NibBadge } from '../../../kit'

import { useMeasuredCssVar } from '../../../lib/composables/use-measured-css-var'
import type { RenderedRule } from '../../../lib/rules/rules.data'
import { familyEmblem }      from '../../../lib/shared/emblems'

export const RuleCard = defineComponent({
  name  : 'RuleCard',
  props : {
    clickable : { default: true, type: Boolean },
    rule      : { required: true, type: Object as PropType<RenderedRule> }
  },
  setup(props, { slots }) {
    const article = ref<HTMLElement | null>(null)
    const caption = ref<HTMLElement | null>(null)

    if (!props.clickable) {
      useMeasuredCssVar({
        measure: () => {
          const c = caption.value
          if (!c) return null
          const range = document.createRange()
          range.selectNodeContents(c)
          const lines = new Map<number, { left: number, right: number }>()
          for (const rect of range.getClientRects()) {
            const top   = Math.round(rect.top)
            const entry = lines.get(top) ?? { left: Infinity, right: -Infinity }
            entry.left  = Math.min(entry.left,  rect.left)
            entry.right = Math.max(entry.right, rect.right)
            lines.set(top, entry)
          }
          let widest = 0
          for (const { left, right } of lines.values()) {
            widest = Math.max(widest, right - left)
          }
          return widest > 0 ? widest : null
        },
        propName : '--rule-card-caption-width',
        target   : article,
        triggers : [() => props.rule.slug]
      })
    }

    return () => (
      <article
        class="rule-card gap-x-16 gap-y-6 px-16 pt-12 pb-14 max-mobile:gap-x-8"
        data-family={props.rule.family}
        ref={article}
      >
        {props.clickable ? <a aria-label={props.rule.name} class="rule-card-cover" href={props.rule.href} /> : null}
        {slots.header ? (
          <div class="rule-card-header pointer-events-none relative z-2">
            {slots.header?.()}
          </div>
        ) : null}
        <span class="rule-card-mark nib-tag pointer-events-none relative z-2">
          <NibBadge emblem={familyEmblem(props.rule.family)} size={14} />
          <span aria-hidden="true" class="middle-dot">·</span>
          <span class="kicker">{props.rule.familyLabel}</span>
        </span>
        <div class="rule-card-body pointer-events-none contents">
          <h3
            class="rule-card-slug m-0 min-w-0 border-none font-mono text-0-92rem leading-1-3 font-600 -tracking-0-01em text-text-1"
            title={props.rule.slug}
          >
            <MiddleEllipsis text={props.rule.slug} />
          </h3>
          <p class="rule-card-caption m-0 p-0 text-left text-0-875rem leading-1-6 text-text-2" innerHTML={props.rule.captionHtml} ref={caption} />
        </div>
      </article>
    )
  }
})
