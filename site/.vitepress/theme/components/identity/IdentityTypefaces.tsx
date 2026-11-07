import { defaultWindow, useEventListener }                     from '@vueuse/core'
import { computed, defineComponent, h, onMounted, shallowRef } from 'vue'
import type { VNodeChild }                                     from 'vue'

import { Tabs }                                         from '../../../kit'
import type { TabsScope }                               from '../../../kit'
import { readType, TYPE_ROLES }                         from '../../../lib/identity/typefaces'
import { formatFolio }                                  from '../../../lib/shared/numerals'
import { IdentityCaption, IdentityCopy, IdentityPanel } from './IdentityParts'

import type { TypeReading, TypeRoleId } from '../../../lib/identity/typefaces'

// Lists the glyph rows set in the chosen role's face.
const GLYPHS = [['Figures', '0123456789'], ['Marks', '& § ¶ † ‡']] as const

const folio = (id: TypeRoleId): string => `№ ${formatFolio(TYPE_ROLES.findIndex(role => role.id === id) + 1)}`

const sentence = (label: string, reading: TypeReading): string =>
  `The ${label.toLowerCase()} takes ${reading.face} at weight ${reading.weight} and ${reading.size}px${reading.style === 'italic' ? ', in italic' : ''}.`

// Sets each typographic role in the site's own styles in the pane's in-place
// block, read back on mount and on each resize into an index grouped by the
// face each role computes. The chosen role's pane gives its weight, size, and
// tracking, the tokens and the selector that set them, a sample at its true
// size, the role marked among the others in place, and the face's figures and
// marks.
export const IdentityTypefaces = defineComponent({
  name : 'IdentityTypefaces',
  setup() {
    const page     = shallowRef<HTMLElement | null>(null)
    const readings = shallowRef<Partial<Record<TypeRoleId, TypeReading>>>({})
    const chosen   = shallowRef<TypeRoleId>(TYPE_ROLES[0].id)
    const faces    = computed(() => Object.entries(Object.groupBy(
      TYPE_ROLES.filter(role => readings.value[role.id]),
      role => readings.value[role.id]!.face
    )))
    const role     = computed(() => TYPE_ROLES.find(each => each.id === chosen.value)!)
    const reading  = computed(() => readings.value[chosen.value])

    const read = (): void => {
      readings.value = Object.fromEntries(TYPE_ROLES.map(each =>
        [each.id, readType(page.value!.querySelector<HTMLElement>(`[data-type-role="${each.id}"]`)!)]))
    }

    onMounted(read)
    useEventListener(defaultWindow, 'resize', read)

    const place = (id: TypeRoleId, ...after: VNodeChild[]) => {
      const each = TYPE_ROLES.find(one => one.id === id)!
      return h(each.tag, { 'class': ['m-0', each.class], 'data-marked': chosen.value === id || undefined, 'data-type-role': id }, [each.sample, ...after])
    }

    const figures = (shown: TypeReading) => (
      <dl class="m-0 flex flex-wrap gap-x-1-5rem gap-y-8">
        {[['Weight', shown.weight], ['Size', `${shown.size}px`], ...shown.tracking === 'normal' ? [] : [['Tracking', shown.tracking]]].map(([label, value]) => (
          <div class="grid" key={label}>
            <dt class="order-last mt-4 kicker">{label}</dt>
            <dd class="m-0 font-display text-1-8rem leading-1-1 font-500 text-text-1 italic tabular-nums">{value}</dd>
          </div>
        ))}
      </dl>
    )

    const glyphs = (shown: TypeReading) => (
      <dl class="mt-16 mb-0 grid grid-cols-max-1fr items-baseline gap-x-1-5rem gap-y-6 border-t-1 border-t-divider pt-12">
        {GLYPHS.map(([label, set]) => [
          <dt class="kicker" key={`${label}-term`}>{label}</dt>,
          <dd class="m-0 truncate text-1-5rem text-text-1" key={label} style={{ fontFamily: shown.family, fontWeight: shown.weight }}>{set}</dd>
        ])}
      </dl>
    )

    return () => (
      <div>
        <IdentityPanel source="by face" title="Roles">
          {{
            aside   : () => <span class="kicker">{reading.value?.face}</span>,
            default : () => (
              <Tabs label="Typographic roles" modelValue={chosen.value} onUpdate:modelValue={(next: TypeRoleId) => { chosen.value = next }} orientation="vertical">
                {{
                  default: ({ content, list, trigger }: TabsScope<TypeRoleId>) => (
                    <div class="identity-faces">
                      <div {...list} class="identity-face-index grid content-start py-12">
                        {faces.value.map(([face, roles]) => [
                          <span aria-hidden="true" class="mx-18 mt-8 mb-4 block kicker" key={face}>{face}</span>,
                          ...roles!.map(each => (
                            <button key={each.id} {...trigger(each.id)} aria-controls={undefined} class="identity-pick identity-face-tab">
                              <span class="font-mono text-kicker-sm tracking-kicker uppercase tabular-nums">{folio(each.id)}</span>
                              <span class="identity-pick-name font-display text-0-98rem font-500 text-text-2 italic">{each.label}</span>
                            </button>
                          ))
                        ])}
                      </div>
                      <div {...content(chosen.value)} class="min-w-0 px-20 pt-16 pb-18">
                        {reading.value && [
                          figures(reading.value),
                          <div class="mt-12 flex flex-wrap gap-6" key="tokens">
                            {reading.value.tokens.map(token => <IdentityCopy key={token} label={`var(${token})`} value={`var(${token})`} />)}
                            <IdentityCopy label={`the selector ${reading.value.selector}`} value={reading.value.selector} />
                          </div>,
                          <p
                            class="mt-16 mb-0 truncate text-text-1"
                            key="sample"
                            style={{
                              fontFamily    : reading.value.family,
                              fontSize      : `${reading.value.size}px`,
                              fontStyle     : reading.value.style,
                              fontWeight    : reading.value.weight,
                              letterSpacing : reading.value.tracking
                            }}
                          >{role.value.sample}</p>
                        ]}
                        <div class="mt-16 border-t-1 border-t-divider pt-12">
                          <span class="kicker">In place</span>
                          <div aria-hidden="true" class="identity-in-place mt-8 grid content-start gap-6" ref={page}>
                            {place('kicker')}
                            {place('h1')}
                            {place('h2')}
                            {place('h3')}
                            {place('body', ' ', place('aside'), ' ', place('code'))}
                            <div class="flex flex-wrap items-baseline gap-x-12">
                              <table class="identity-table m-0 w-max"><thead><tr>{place('th')}</tr></thead></table>
                              {place('callout')}
                              {place('folio')}
                            </div>
                          </div>
                        </div>
                        {reading.value && glyphs(reading.value)}
                      </div>
                    </div>
                  )
                }}
              </Tabs>
            )
          }}
        </IdentityPanel>
        <IdentityCaption>{{ default: () => reading.value ? sentence(role.value.label, reading.value) : '' }}</IdentityCaption>
      </div>
    )
  }
})
