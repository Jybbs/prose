import { computed, defineComponent, ref } from 'vue'

import { ScrollRegion, Tabs } from '../../../kit'
import type { TabsScope }     from '../../../kit'
import { data }               from '../../../lib/identity/identity.data'
import { ROLES, SCHEME_TEXT } from '../../../lib/identity/roles'
import { IdentityCopyChip }   from './IdentityParts'

import type { RoleId } from '../../../lib/identity/roles'
import type { Scheme } from '../../../lib/shared/palette'

// Sets a short page passage in either scheme beside a legend naming every color
// role it draws with, pointing at a role marking each place the passage uses it.
export const IdentityRoles = defineComponent({
  name : 'IdentityRoles',
  setup() {
    const scheme = ref<Scheme>('light')
    const marked = ref<RoleId>(ROLES[0].id)
    const tokens = computed(() => Object.fromEntries(ROLES.map(role => [role.token, data.roles[role.id][scheme.value]])))
    const role   = (id: RoleId) => ({ 'data-marked': marked.value === id || undefined, 'data-role': id })

    return () => (
      <div class="my-1-5rem grid gap-1-25rem">
        <Tabs label="Passage scheme" modelValue={scheme.value} onUpdate:modelValue={(next: Scheme) => { scheme.value = next }}>
          {{
            default: ({ list, trigger }: TabsScope<Scheme>) => (
              <div {...list} class="inline-flex justify-self-start rounded-sm border-1 border-divider bg-bg p-2">
                {data.schemes.toReversed().map(each => (
                  <button key={each} {...trigger(each)} aria-controls={undefined} class="toggle-pill">{SCHEME_TEXT[each]}</button>
                ))}
              </div>
            )
          }}
        </Tabs>
        <div class="grid grid-cols-2fr-1fr gap-1-25rem max-desktop:grid-cols-1fr">
          <div
            {...role('page')}
            class="identity-mark grid gap-0-75rem rounded-sm border-1 border-divider bg-bg p-1-25rem text-text-1"
            style={tokens.value}
          >
            <p {...role('text-1')} class="identity-mark m-0 font-display text-1-5rem leading-1-2 text-text-1">Reading at a Glance</p>
            <p {...role('text-2')} class="identity-mark m-0 text-sm text-text-2">Guide · Formatting a Project</p>
            <hr {...role('divider')} class="identity-mark m-0 border-0 border-t-1 border-t-divider" />
            <p class="m-0 leading-1-65">
              <span {...role('text-1')} class="identity-mark">Run </span>
              <code {...role('code-ground')} class="identity-mark code-chip px-4 font-mono text-code">
                <span {...role('code-ink')} class="identity-mark text-code-ink">prose format</span>
              </code>
              <span {...role('text-1')} class="identity-mark"> over a project and its columns line up, </span>
              <em {...role('aside')} class="identity-mark text-aside">set once and read many times</em>
              <span {...role('text-1')} class="identity-mark">, as the </span>
              <span {...role('link')} class="identity-mark text-brand-1 underline">configuration reference</span>
              <span {...role('text-1')} class="identity-mark"> sets out.</span>
            </p>
            <div {...role('soft')} class="identity-mark grid gap-0-5rem rounded-sm bg-bg-soft p-0-75rem">
              <ScrollRegion as="pre" {...role('code-block')} class="identity-mark m-0 overflow-auto rounded-sm bg-code-block p-0-75rem font-mono text-code">
                {{ default: () => <span {...role('code-ink')} class="identity-mark text-code-ink">{'width  = 88\nheight = 24'}</span> }}
              </ScrollRegion>
              <p {...role('text-3')} class="identity-mark m-0 text-xs text-text-3">A config the formatter aligned.</p>
            </div>
          </div>
          <Tabs label="Color roles" modelValue={marked.value} onUpdate:modelValue={(next: RoleId) => { marked.value = next }} orientation="vertical">
            {{
              default: ({ list, trigger }: TabsScope<RoleId>) => (
                <div {...list} class="grid content-start">
                  {ROLES.map(each => (
                    <button
                      key={each.id}
                      {...trigger(each.id)}
                      aria-controls={undefined}
                      class="identity-legend grid grid-cols-14-1fr-auto items-center gap-x-8"
                    >
                      <span
                        aria-hidden="true"
                        class="size-14 rounded-sm border-1 border-divider"
                        style={{ background: data.roles[each.id][scheme.value] }}
                      />
                      <span class="grid">
                        <span class="text-sm text-text-1">{each.label}</span>
                        <span class="font-mono text-meta text-text-3">{each.token}</span>
                      </span>
                      <span class="font-mono text-xs text-text-2">{data.roles[each.id][scheme.value]}</span>
                    </button>
                  ))}
                </div>
              )
            }}
          </Tabs>
        </div>
        <div class="flex flex-wrap gap-8">
          <IdentityCopyChip label="the marked role's token" value={ROLES.find(each => each.id === marked.value)!.token} />
          <IdentityCopyChip label="the marked role's value" value={data.roles[marked.value][scheme.value]} />
        </div>
      </div>
    )
  }
})
