import { defineComponent, ref } from 'vue'

import type { Declaration }  from '../catalog'
import { Listbox }           from './Listbox'
import type { ListboxScope } from './Listbox'

const OPTIONS = [{ id: 'jsonl' }, { id: 'sarif' }, { id: 'text' }]

const Example = defineComponent(() => {
  const value = ref('text')

  return () => (
    <Listbox label="Output format" modelValue={value.value} onUpdate:modelValue={next => { value.value = next }} options={OPTIONS}>
      {{
        default: ({ content, item, positioner, trigger }: ListboxScope<(typeof OPTIONS)[number]>) => (
          <span class="relative inline-block">
            <button {...trigger} class="border-none px-2 font-mono text-sm text-brand-1" type="button">{value.value}</button>
            <div {...positioner}>
              <ul {...content} class="z-10000 m-0 min-w-max list-none rounded-sm border-1 border-divider bg-bg p-4 shadow-tooltip">
                {OPTIONS.map(option => (
                  <li class="mt-0" key={option.id} role="presentation">
                    <button {...item(option)} class="w-full rounded-sm border-none px-11 py-7 text-start font-mono text-sm data-highlighted:bg-bg-soft" type="button">{option.id}</button>
                  </li>
                ))}
              </ul>
            </div>
          </span>
        )
      }}
    </Listbox>
  )
})

export const declaration: Declaration = {
  examples : [
    { label: 'A click opens it, and the arrow keys move through it', render: () => <Example /> }
  ],
  use      : 'Picks one value from a short list, positioned against its trigger, leaving its markup to the caller.'
}
