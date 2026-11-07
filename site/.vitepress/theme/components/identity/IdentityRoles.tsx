import { defineComponent, shallowRef } from 'vue'

import { ScrollRegion, Tabs }             from '../../../kit'
import type { TabsScope }                 from '../../../kit'
import { useRaised }                      from '../../../lib/composables/use-raised'
import { data }                           from '../../../lib/identity/identity.data'
import * as roles                         from '../../../lib/identity/roles'
import { KINDS }                          from '../../../lib/markdown/callout-plugin'
import { IdentityCaption, IdentityPanel } from './IdentityParts'

import type { Role }   from '../../../lib/identity/roles'
import type { Scheme } from '../../../lib/shared/palette'

const FLOOR = Object.fromEntries(roles.LEVELS)

const passes = (role: Role, scheme: Scheme): boolean => role.contrast[scheme].ratio >= FLOOR[role.judged]!

const kinds = data.roles.filter(role => role.group === 'marks').map(role => {
  const kind = role.id.replace('callout-', '') as keyof typeof KINDS
  return { kind, role, ...KINDS[kind] }
})

// Sets a short page passage in the light scheme or the dark beside a legend of
// every color role it draws with, each role's contrast judged at its level, and
// pointing at, focusing, or pinning a role outlines every place the passage
// uses it.
export const IdentityRoles = defineComponent({
  name : 'IdentityRoles',
  setup() {
    const scheme                = shallowRef<Scheme>('light')
    const { on, pinned, shown } = useRaised<string>()
    const mark                  = (id: string) => ({ 'class': 'identity-mark', 'data-marked': shown.value === id || undefined, 'data-role': id })

    return () => (
      <div>
        <IdentityPanel source="one page" title="Roles">
          {{
            aside   : () => (
              <Tabs label="Passage scheme" modelValue={scheme.value} onUpdate:modelValue={(next: Scheme) => { scheme.value = next }}>
                {{
                  default: ({ list, trigger }: TabsScope<Scheme>) => (
                    <div {...list} class="toggle-row justify-self-start">
                      {roles.SCHEME_ORDER.map(each => (
                        <button key={each} {...trigger(each)} aria-controls={undefined} class="toggle-pill">{roles.SCHEME_TEXT[each]}</button>
                      ))}
                    </div>
                  )
                }}
              </Tabs>
            ),
            default : () => (
              <div class="identity-roles">
                <div {...mark('page')} class="bg-bg p-20 text-text-1" style={data.passage[scheme.value]}>
                  <p class="m-0 leading-1-65">
                    <span {...mark('text-1')}>Body text sits on the page ground, </span>
                    <span {...mark('link')} class="nib-stroke pb-1">a link</span>
                    <span {...mark('text-1')}> carries the link rule and turns to </span>
                    <span {...mark('link-hover')} class="text-brand-2">the link hover</span>
                    <span {...mark('text-1')}> under the pointer, </span>
                    <code {...mark('code-ground')}>inline code</code>
                    <span {...mark('text-1')}> sits on its own ground, and </span>
                    <em {...mark('aside')} class="text-aside">an italic aside</em>
                    <span {...mark('text-1')}> takes the aside color.</span>
                  </p>
                  <ScrollRegion as="pre" {...mark('code-block')} class="mt-16 mb-0 overflow-auto rounded-sm bg-code-block px-16 py-12 font-mono text-code text-code-ink">
                    {{ default: () => 'prose check --output-format sarif .' }}
                  </ScrollRegion>
                  <table class="identity-table mt-16 mb-0 w-full">
                    <thead>
                      <tr><th {...mark('soft')}>Format</th><th {...mark('text-2')}>Output</th></tr>
                    </thead>
                    <tbody>
                      <tr><td><code>text</code></td><td {...mark('text-1')}>Snippets for a terminal</td></tr>
                      <tr {...mark('soft')}><td><code>sarif</code></td><td>A SARIF 2.1.0 document</td></tr>
                    </tbody>
                  </table>
                  <p {...mark('text-3')} class="mt-12 mb-0 text-xs text-text-3">Each format prints to stdout.</p>
                  <div class="identity-callouts mt-16">
                    {kinds.map(({ kind, label, mark: glyph, role }) => (
                      <div class={['custom-block', kind]} key={kind}>
                        <p class="custom-block-title">
                          <span {...mark(role.id)} aria-hidden="true" class="callout-mark">{glyph}</span>
                          <span {...mark('text-2')} class="callout-kind">{label}</span>
                        </p>
                        <p {...mark('text-1')}>{'Its mark reads '}<code>{role.token}</code>.</p>
                      </div>
                    ))}
                  </div>
                </div>
                <div class="identity-side py-8">
                  {Object.entries(roles.ROLE_GROUPS).map(([group, title]) => (
                    <div key={group}>
                      <span class="mx-14 mt-8 mb-4 block kicker">{title}</span>
                      {data.roles.filter(role => role.group === group).map(role => (
                        <button
                          aria-pressed={pinned.value === role.id}
                          class="identity-pick identity-legend"
                          key={role.id}
                          type="button"
                          {...on(role.id)}
                        >
                          <span aria-hidden="true" class="identity-proof size-12" style={{ background: role.value[scheme.value] }} />
                          <span class="identity-pick-name font-display text-0-98rem font-500 text-text-2 italic">{role.label}</span>
                          <span class="font-mono text-xs text-text-1 tabular-nums">{`${role.contrast[scheme.value].ratio.toFixed(2)}:1`}</span>
                          <span class="font-display text-sm text-text-2 italic">{passes(role, scheme.value) ? 'pass' : 'fail'}</span>
                          <span class="sr-only">{`, ${role.token}, ${role.value[scheme.value]}`}</span>
                        </button>
                      ))}
                    </div>
                  ))}
                </div>
              </div>
            )
          }}
        </IdentityPanel>
        <IdentityCaption>
          {{ default: () => `The passage is drawn in the light scheme whatever the page around it is set to, and the switch above it turns it to the other. Each text role is judged at ${FLOOR.text}:1 against the page ground, each callout mark and the link rule at ${FLOOR.mark}:1, and each ground by the text it carries. Pointing at, focusing, or pinning a role marks every place the passage uses it.` }}
        </IdentityCaption>
      </div>
    )
  }
})
