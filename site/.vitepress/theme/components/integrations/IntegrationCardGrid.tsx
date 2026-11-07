import { defineComponent } from 'vue'

import { data as cards } from '../../../lib/integrations/integrations.data'
import { InlineProse }   from '../base/InlineProse'

export const IntegrationCardGrid = defineComponent({
  name: 'IntegrationCardGrid',
  setup() {
    return () => (
      <div class="my-1-5rem grid grid-cols-fill-260 gap-1rem">
        {cards.map(card => (
          <article
            class="int-card group relative rounded-sm border-1 border-divider bg-bg-soft px-1-1rem pt-1rem pb-0-85rem text-text-1 transition-colors hover:border-ube hover:bg-bg has-focus-visible:border-ube has-focus-visible:bg-bg"
            key={card.href}
          >
            <p class="mt-0 mb-0-3rem font-mono text-kicker-sm tracking-label-loose text-text-3 uppercase">{card.tagline}</p>
            <p class="mt-0 mb-0-5rem border-none font-display text-1-2rem font-500 text-text-1 italic">
              <a class="text-inherit no-underline after:absolute after:inset-0" href={card.href}>{card.title}</a>
            </p>
            <p class="int-body mt-0 mb-0-7rem text-0-88rem leading-1-45 text-text-2">
              <InlineProse nodes={card.summaryNodes} />
            </p>
            <p
              aria-hidden="true"
              class="m-0 font-mono text-kicker tracking-meta text-text-2 transition-colors group-hover:text-link group-has-focus-visible:text-link"
            >Open →</p>
          </article>
        ))}
      </div>
    )
  }
})
