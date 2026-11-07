import { defineComponent, ref } from 'vue'

import type { Declaration } from '../catalog'
import { Tabs }             from './Tabs'
import type { TabsScope }   from './Tabs'

const VALUES = ['format', 'check'] as const

const Example = defineComponent(() => {
  const value = ref<(typeof VALUES)[number]>('format')

  return () => (
    <Tabs label="Commands" modelValue={value.value} onUpdate:modelValue={next => { value.value = next }}>
      {{
        default: ({ content, list, trigger }: TabsScope<(typeof VALUES)[number]>) => (
          <div>
            <div {...list} class="flex gap-4">
              {VALUES.map(tab => (
                <button
                  {...trigger(tab)}
                  class="rounded-sm border-none px-7 py-2 font-mono text-sm text-text-2 aria-selected:text-brand-1"
                  type="button"
                >{tab}</button>
              ))}
            </div>
            {VALUES.map(tab => <p {...content(tab)} class="mt-4 mb-0 font-mono text-sm">prose {tab}</p>)}
          </div>
        )
      }}
    </Tabs>
  )
})

export const declaration: Declaration = {
  examples : [
    { label: 'Each tab selects on a click, a hover, or a focus', render: () => <Example /> }
  ],
  use      : 'Selects one panel from a tab list on a click, a hover, or a focus, leaving its markup to the caller.'
}
