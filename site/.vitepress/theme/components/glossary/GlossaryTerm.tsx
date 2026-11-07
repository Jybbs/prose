import { useRoute }                  from 'vitepress'
import { computed, defineComponent } from 'vue'

import { nib, Popup }             from '../../../kit'
import type { PopupTriggerScope } from '../../../kit'

import { useAriaHidden }    from '../../../lib/composables/use-aria-hidden'
import { data as glossary } from '../../../lib/glossary/glossary.data'
import { familyEmblem }     from '../../../lib/shared/emblems'
import { lookup }           from '../../../lib/shared/lookup'
import { stripSuffix }      from '../../../lib/shared/strip-suffix'

const LINK_CLASS = '-mx-16 mt-12 -mb-12 block border-t-1 border-t-divider px-16 py-6 '
                 + 'text-right font-mono text-label tracking-meta text-text-1 uppercase no-underline'

export const GlossaryTerm = defineComponent({
  name  : 'GlossaryTerm',
  props : {
    slug : { required: true, type: String }
  },
  setup(props, { slots }) {
    const ariaHidden = useAriaHidden()
    const entry      = lookup(glossary.entries, props.slug, 'Glossary entry')
    const route      = useRoute()

    const families = entry.families

    // Heads the tooltip on one line with the entry's title and, at its right
    // end, each family's nib, a middle dot, and the Glossary kicker.
    const head = [
      `<div class="glossary-tooltip-title m-0 min-w-0 font-display text-1-2rem leading-1-15 font-500 whitespace-nowrap text-text-1">${entry.slug}</div>`,
      '<div class="nib-tag">',
      ...families.map(family => {
        const emblem = familyEmblem(family)
        return `<span class="glossary-tooltip-nib" data-family="${family}">${nib.nibBadgeHtml(emblem, 14, emblem.name)}</span>`
      }),
      families.length > 0 ? '<span class="middle-dot" aria-hidden="true">·</span>' : '',
      '<span class="kicker">Glossary</span>',
      '</div>'
    ].join('')

    const tooltipHtml = computed(() => {
      const parts = [
        `<div class="glossary-tooltip-head flex items-center justify-between gap-16">${head}</div>`,
        `<div class="glossary-tooltip-body mt-8 text-text-2 italic">${entry.definitionHtml}</div>`
      ]
      if (showLink.value) {
        parts.push(`<a href="${entry.href}" class="${LINK_CLASS}"><span class="nib-stroke">Read more →</span></a>`)
      }
      return parts.join('')
    })

    const showLink = computed(() => {
      if (!entry.href) return false
      if (entry.href.includes('#')) return true
      const current = stripSuffix(stripSuffix(route.path, '.html'), '/')
      return current !== stripSuffix(entry.href, '/')
    })

    return () => (
      <Popup label={entry.slug} placement="top" theme="glossary">
        {{
          default: () => <div innerHTML={tooltipHtml.value} />,
          trigger: ({ props: trigger }: PopupTriggerScope) => (
            <span
              {...trigger}
              class="glossary-anchor underline-draw"
              tabindex={ariaHidden.value ? -1 : 0}
            >
              {slots.default?.()}
            </span>
          )
        }}
      </Popup>
    )
  }
})
