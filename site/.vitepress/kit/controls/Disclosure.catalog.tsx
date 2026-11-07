import { defineComponent, ref } from 'vue'

import type { Declaration }     from '../catalog'
import { Disclosure }           from './Disclosure'
import type { DisclosureScope } from './Disclosure'

const Example = defineComponent(() => {
  const open = ref(false)

  return () => (
    <Disclosure onUpdate:open={next => { open.value = next }} open={open.value}>
      {{
        default: ({ content, trigger }: DisclosureScope) => (
          <div>
            <button {...trigger} class="border-none p-0 font-mono text-sm text-brand-1" type="button">
              {open.value ? 'Hide the note' : 'Show the note'}
            </button>
            <p {...content} class="mt-4 mb-0 text-sm text-text-2">A disclosed note.</p>
          </div>
        )
      }}
    </Disclosure>
  )
})

export const declaration: Declaration = {
  examples : [
    { label: 'A click or a key toggles it', render: () => <Example /> }
  ],
  use      : 'Opens and closes one block from one trigger, leaving its markup and its open state to the caller.'
}
