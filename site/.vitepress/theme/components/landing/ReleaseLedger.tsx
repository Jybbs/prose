import { computed, defineComponent } from 'vue'
import type { PropType }             from 'vue'

import type { PyPIRelease } from '../../../lib/landing/pypi-releases.data'
import { externalAttrs }    from '../../../lib/shared/links'

export const ReleaseLedger = defineComponent({
  name  : 'ReleaseLedger',
  props : {
    releases : { required: true, type: Array as PropType<readonly PyPIRelease[]> }
  },
  setup(props) {
    const groupedByYear = computed(() =>
      Array.from(Map.groupBy(props.releases, r => r.year),
                 ([year, items]) => ({ items, year }))
    )

    return () => (
      <div class="landing-cta-scroll relative -ms-10 min-h-0 flex-1 scrollbar-thumb-ube-60-panel-edge scrollbar-track-transparent scrollbar-gutter-stable self-stretch overflow-x-hidden overflow-y-auto">
        {groupedByYear.value.map(group => (
          <div class="ps-14 not-first:mt-6" key={group.year}>
            <p class="sticky top-0 z-1 bg-panel-bg px-8 pt-4 pb-2 font-mono text-kicker-sm tracking-display-wide text-ube-page-45 uppercase">{group.year}</p>
            <ol class="flex flex-col gap-1">
              {group.items.map(r => (
                <li key={r.version}>
                  <a
                    href={r.url}
                    {...externalAttrs(r.url)}
                    class="grid grid-cols-auto-minmax-12-1fr-auto items-baseline gap-6 rounded-sm px-8 py-4 font-mono text-text-1 transition-colors hover:bg-stamp-accent-medium-bg hover:text-stamp-ink"
                  >
                    <span class="font-display text-0-98rem text-stamp-ink italic">{r.version}</span>
                    <span aria-hidden="true" class="mx-2 leader" />
                    <span class="text-0-66rem tracking-0-12em text-text-3">{r.month}</span>
                  </a>
                </li>
              ))}
            </ol>
          </div>
        ))}
      </div>
    )
  }
})
