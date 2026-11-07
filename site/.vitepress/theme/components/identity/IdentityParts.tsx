import { useClipboard }                    from '@vueuse/core'
import { defineComponent, ref, useId }     from 'vue'
import type { PropType, SlotsType, VNode } from 'vue'

import { Disclosure }           from '../../../kit'
import type { DisclosureScope } from '../../../kit'

// Frames a specimen in the site's masthead card, a kicker naming it and its
// source over a double rule, with the `aside` slot set at the masthead's end.
export const IdentityPanel = defineComponent({
  name  : 'IdentityPanel',
  props : {
    source : { required: true, type: String },
    title  : { required: true, type: String }
  },
  slots : Object as SlotsType<{ aside?: () => VNode | VNode[], default: () => VNode | VNode[] }>,
  setup(props, { slots }) {
    const id = useId()
    return () => (
      <section aria-labelledby={id} class="mt-1-5rem overflow-hidden panel bg-bg">
        <header class="flex flex-wrap items-baseline justify-between gap-x-16 gap-y-6 px-18 pt-13 pb-12 border-b-double-divider">
          <span class="kicker" id={id}>
            {`${props.title} · `}
            <span class="tracking-normal text-text-2 normal-case">{props.source}</span>
          </span>
          {slots.aside?.()}
        </header>
        {slots.default()}
      </section>
    )
  }
})

// Sets a value as a button copying it, showing `text` in the value's place
// where one is given, the word "Copied" drawn over it in place once the copy
// lands, the `chip` variant boxing it and the `plain` one underlining it with
// dots.
export const IdentityCopy = defineComponent({
  name  : 'IdentityCopy',
  props : {
    label   : { required: true, type: String },
    text    : String,
    value   : { required: true, type: String },
    variant : { default: 'chip', type: String as PropType<'chip' | 'plain'> }
  },
  setup(props) {
    const { copied, copy } = useClipboard()
    return () => (
      <button
        aria-label={copied.value ? `Copied ${props.label}` : `Copy ${props.label}`}
        class="identity-copy cursor-copy font-mono text-xs whitespace-nowrap text-text-1 tabular-nums focus-visible:focus-ring focus-visible:outline-offset-2"
        data-copied={copied.value || undefined}
        data-variant={props.variant}
        onClick={() => copy(props.value)}
        type="button"
      >
        <span class="identity-copy-value">{props.text ?? props.value}</span>
        <span aria-hidden="true" class="identity-copy-done text-text-2">Copied</span>
      </button>
    )
  }
})

// Opens one row of a specimen's entry under a summary line, the label in the
// entry's key column, the one-line summary beside it, and a chevron turning as
// the row opens or closes in place.
export const IdentityRow = defineComponent({
  name  : 'IdentityRow',
  props : {
    label   : { required: true, type: String },
    open    : Boolean,
    summary : { required: true, type: String }
  },
  setup(props, { slots }) {
    const open = ref(props.open)
    return () => (
      <Disclosure open={open.value} onUpdate:open={(next: boolean) => { open.value = next }}>
        {{
          default: ({ content, trigger }: DisclosureScope) => (
            <div class="border-t-1 border-t-divider">
              <button {...trigger} class="identity-row-summary group w-full cursor-pointer border-none bg-transparent px-0 py-9 text-left focus-visible:focus-ring focus-visible:outline-offset-2">
                <span class="kicker">{props.label}</span>
                <span class="min-w-0 truncate font-mono text-xs text-text-1 tabular-nums group-aria-expanded:text-text-3">{props.summary}</span>
                <span aria-hidden="true" class="identity-chevron" />
              </button>
              <div {...content} class="identity-row-body pb-12">{slots.default?.()}</div>
            </div>
          )
        }}
      </Disclosure>
    )
  }
})

// Holds every value a specimen shows as plain text behind a disclosure named
// `label`, closed at rest.
export const IdentityPlain = defineComponent({
  name  : 'IdentityPlain',
  props : { label: { required: true, type: String } },
  setup(props, { slots }) {
    const open = ref(false)
    return () => (
      <Disclosure open={open.value} onUpdate:open={(next: boolean) => { open.value = next }}>
        {{
          default: ({ content, trigger }: DisclosureScope) => (
            <div class="mt-14">
              <button {...trigger} class="group inline-flex cursor-pointer items-center gap-9 border-none bg-transparent px-0 py-4 kicker focus-visible:focus-ring focus-visible:outline-offset-2">
                <span aria-hidden="true" class="identity-plain-mark" />
                {props.label}
              </button>
              <div {...content} class="mt-8 text-xs leading-1-6 text-text-2">{slots.default?.()}</div>
            </div>
          )
        }}
      </Disclosure>
    )
  }
})

// Closes a specimen on the one sentence stating what the reader is looking at.
export const IdentityCaption = defineComponent({
  name : 'IdentityCaption',
  setup(_, { slots }) {
    return () => <p class="mt-16 mb-0 text-sm leading-1-6 text-text-2">{slots.default?.()}</p>
  }
})
