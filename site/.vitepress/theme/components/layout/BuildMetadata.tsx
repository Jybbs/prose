import { useData }                   from 'vitepress'
import { computed, defineComponent } from 'vue'

import { data as build } from '../../../lib/shared/build.data'
import { data as rules } from '../../../lib/rules/rules.data'

interface Item {
  code   ?: boolean
  label   : string
  value   : string | number
}

export const BuildMetadata = defineComponent({
  name: 'BuildMetadata',
  setup() {
    const { page } = useData()

    const lastUpdated = computed(() => {
      const ts = page.value.lastUpdated
      return ts ? new Date(ts).toLocaleDateString('en-CA', { timeZone: 'UTC' }) : ''
    })

    const items = computed((): Item[] => [
      {             label: 'Version',  value: build.version       },
      { code: true, label: 'Commit',   value: build.gitSha        },
      {             label: 'Rules',    value: rules.list.length   },
      {             label: 'Fixtures', value: build.fixtureCount  },
      ...(lastUpdated.value ? [{ label: 'Updated', value: lastUpdated.value }] : [])
    ])

    return () => (
      <section
        aria-label="Build"
        class="mx-auto mt-24 flex max-w-1120 flex-wrap justify-center gap-x-52 gap-y-16 border-t-1 border-t-divider px-18 pt-18 pb-14 font-mono tracking-folio"
      >
        {items.value.map(item => (
          <span class="flex flex-col items-center gap-1" key={item.label}>
            <span class="text-md leading-none text-text-1">
              {item.code
                ? <code>{item.value}</code>
                : item.value}
            </span>
            <span class="text-meta tracking-0-12em text-text-3 uppercase">{item.label}</span>
          </span>
        ))}
      </section>
    )
  }
})
