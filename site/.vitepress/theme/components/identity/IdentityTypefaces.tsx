import { defaultWindow, useEventListener }              from '@vueuse/core'
import { computed, defineComponent, h, onMounted, ref } from 'vue'
import type { VNodeChild }                              from 'vue'

import { Tabs }                          from '../../../kit'
import type { TabsScope }                from '../../../kit'
import { readType, TYPE_ROLES }          from '../../../lib/identity/typefaces'
import { IdentityCopyChip, IdentityRow } from './IdentityParts'

import type { TypeReading, TypeRoleId } from '../../../lib/identity/typefaces'

const GLYPHS = '0123456789 § ¶ † №'

// Sets each typographic role on a small page in the site's own styles, read
// back on mount and on each resize into an index grouped by the face each role
// computes, the chosen role's pane giving its weight, size, and tokens beside
// a line of the face's numerals and reference marks.
export const IdentityTypefaces = defineComponent({
  name : 'IdentityTypefaces',
  setup() {
    const page     = ref<HTMLElement | null>(null)
    const readings = ref<Partial<Record<TypeRoleId, TypeReading>>>({})
    const chosen   = ref<TypeRoleId>(TYPE_ROLES[0].id)
    const faces    = computed(() => Object.entries(Object.groupBy(
      TYPE_ROLES.filter(role => readings.value[role.id]),
      role => readings.value[role.id]!.face
    )))
    const reading  = computed(() => readings.value[chosen.value])

    const read = (): void => {
      readings.value = Object.fromEntries(TYPE_ROLES.map(role =>
        [role.id, readType(page.value!.querySelector<HTMLElement>(`[data-type-role="${role.id}"]`)!)]))
    }

    onMounted(read)
    useEventListener(defaultWindow, 'resize', read)

    const place = (id: TypeRoleId, ...after: VNodeChild[]) => {
      const role = TYPE_ROLES.find(each => each.id === id)!
      return h(role.tag, {
        class            : ['identity-mark', 'm-0', role.class],
        'data-marked'    : chosen.value === id || undefined,
        'data-type-role' : id
      }, [role.sample, ...after])
    }

    return () => (
      <div class="my-1-5rem grid grid-cols-2fr-1fr gap-1-25rem max-desktop:grid-cols-1fr">
        <div aria-hidden="true" class="identity-small-page grid content-start gap-0-75rem rounded-sm border-1 border-divider bg-bg p-1-25rem" ref={page}>
          {place('kicker')}
          {place('h1')}
          {place('h2')}
          {place('h3')}
          {place('body', ' ', place('aside'), ' ', place('code'))}
          <table class="identity-table m-0"><thead><tr>{place('th')}</tr></thead></table>
          {place('folio')}
        </div>
        <div class="grid content-start gap-1-25rem">
          {faces.value.length > 0 && (
            <Tabs label="Typographic roles" modelValue={chosen.value} onUpdate:modelValue={(next: TypeRoleId) => { chosen.value = next }} orientation="vertical">
              {{
                default: ({ content, list, trigger }: TabsScope<TypeRoleId>) => {
                  const shown = reading.value
                  return [
                    <div key="list" {...list} class="grid content-start">
                      {faces.value.map(([face, roles]) => [
                        <span aria-hidden="true" class="kicker mt-0-5rem" key={face}>{face}</span>,
                        ...roles!.map(role => (
                          <button
                            key={role.id}
                            {...trigger(role.id)}
                            aria-controls={undefined}
                            class="identity-legend flex items-baseline justify-between gap-x-8 text-sm text-text-1"
                          >
                            {role.label}
                            <span class="font-mono text-meta text-text-3 tabular-nums">{`${readings.value[role.id]!.size}px`}</span>
                          </button>
                        ))
                      ])}
                    </div>,
                    shown && (
                      <div key="pane" {...content(chosen.value)} class="grid grid-cols-max-1fr gap-x-1-5rem">
                        <IdentityRow label="Figures">{{ default: () => (
                          <dl class="m-0 grid grid-cols-max-1fr gap-x-1rem gap-y-2 text-sm">
                            <dt class="text-text-3">Face</dt><dd class="m-0">{shown.face}</dd>
                            <dt class="text-text-3">Weight</dt><dd class="m-0 tabular-nums">{shown.weight}</dd>
                            <dt class="text-text-3">Size</dt><dd class="m-0 tabular-nums">{`${shown.size}px`}</dd>
                            <dt class="text-text-3">Style</dt><dd class="m-0">{shown.style}</dd>
                            <dt class="text-text-3">Tracking</dt><dd class="m-0">{shown.tracking}</dd>
                          </dl>
                        ) }}</IdentityRow>
                        <IdentityRow label="Tokens">{{ default: () => (
                          <div class="flex flex-wrap gap-8">
                            {shown.tokens.length > 0
                              ? shown.tokens.map(token => <IdentityCopyChip key={token} label={token} value={token} />)
                              : <span class="text-sm text-text-2">The role reads no token for its type.</span>}
                          </div>
                        ) }}</IdentityRow>
                        <IdentityRow label="Marks">{{ default: () => (
                          <p class="m-0 text-1-5rem text-text-1" style={{ fontFamily: shown.family, fontWeight: shown.weight }}>{GLYPHS}</p>
                        ) }}</IdentityRow>
                      </div>
                    )
                  ]
                }
              }}
            </Tabs>
          )}
        </div>
      </div>
    )
  }
})
