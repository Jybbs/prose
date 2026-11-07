import { defineComponent, ref } from 'vue'
import type { PropType }         from 'vue'

import { CopyButton, Disclosure } from '../../../kit'
import type { DisclosureScope }   from '../../../kit'
import { SCHEME_TEXT }            from '../../../lib/identity/roles'

import type { Scheme } from '../../../lib/shared/palette'

// Sets a value in a code chip beside a button copying it, the button naming
// what it copies.
export const IdentityCopyChip = defineComponent({
  name  : 'IdentityCopyChip',
  props : {
    label : { required: true, type: String },
    value : { required: true, type: String }
  },
  setup(props) {
    return () => (
      <span class="identity-copy copy-host relative inline-flex items-center code-chip py-2 pr-32 pl-6 font-mono text-code text-code-ink">
        {props.value}
        <CopyButton label={`Copy ${props.label}`} source={props.value} />
      </span>
    )
  }
})

// Names a scheme in the label column of a row of per-scheme values.
export const IdentitySchemeLabel = defineComponent({
  name  : 'IdentitySchemeLabel',
  props : { scheme: { required: true, type: String as PropType<Scheme> } },
  setup(props) {
    return () => <span class="w-3rem font-mono text-meta text-text-3 uppercase">{SCHEME_TEXT[props.scheme]}</span>
  }
})

// Opens one row of a specimen's pane under a trigger naming it in the pane's
// label column, open until the reader closes it.
export const IdentityRow = defineComponent({
  name  : 'IdentityRow',
  props : { label: { required: true, type: String } },
  setup(props, { slots }) {
    const open = ref(true)
    return () => (
      <Disclosure open={open.value} onUpdate:open={(next: boolean) => { open.value = next }}>
        {{
          default: ({ content, trigger }: DisclosureScope) => (
            <div class="col-span-full grid grid-cols-subgrid border-t-1 border-t-divider py-0-5rem">
              <button
                {...trigger}
                class="flex cursor-pointer items-baseline gap-6 self-start border-none bg-transparent p-0 text-left kicker focus-visible:focus-ring"
              >{props.label}</button>
              <div {...content} class="min-w-0">{slots.default?.()}</div>
            </div>
          )
        }}
      </Disclosure>
    )
  }
})
