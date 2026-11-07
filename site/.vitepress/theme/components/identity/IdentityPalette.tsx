import { useTimeoutFn }                                 from '@vueuse/core'
import { computed, defineComponent, shallowRef, watch } from 'vue'
import { romanize }                                     from 'romans'

import { useRaised }            from '../../../lib/composables/use-raised'
import { data }                 from '../../../lib/identity/identity.data'
import { LEVELS, SCHEME_ORDER } from '../../../lib/identity/roles'
import { arrowIndex }           from '../../../lib/shared/arrow-keys'
import { formatFolio }          from '../../../lib/shared/numerals'
import { IdentityCaption, IdentityCopy, IdentityPanel, IdentityPlain, IdentityRow } from './IdentityParts'

import type { Level }  from '../../../lib/identity/roles'
import type { Swatch } from '../../../lib/identity/swatches'
import type { Scheme } from '../../../lib/shared/palette'

const LEVEL_TEXT: Record<Level, string> = {
  fails : 'fails as text and as a mark',
  mark  : 'passes as a mark but not as text',
  text  : 'passes as text and as a mark'
}

const LEVEL_INK: Record<Level, string> = { fails: 'text-text-3', mark: 'text-text-2', text: 'text-text-1' }

const SCHEME_WORD = (schemes: readonly Scheme[]): string => schemes.length === SCHEME_ORDER.length ? 'both' : schemes[0]!

// Holds the entry of the color the pointer last left for this long, so the
// pointer can cross from the band to the entry and copy a value from it.
const GRACE_MS = 320

const groups = data.groups.map((group, at) => {
  const swatches = data.swatches.filter(swatch => swatch.group === group.id)
  const strips   = group.id === 'brand'
    ? [...swatches.filter(swatch => swatch.mix?.toward === 'black'), swatches[0]!, ...swatches.filter(swatch => swatch.mix?.toward === 'white')]
    : swatches
  return { ...group, numeral: romanize(at + 1), strips }
})

const strips = groups.flatMap(group => group.strips)
const lead   = data.swatches[0]!

const meta = (swatch: Swatch): string => {
  const group = groups.find(each => each.id === swatch.group)!
  return `${group.numeral} · ${group.title} · № ${formatFolio(data.swatches.indexOf(swatch) + 1)}`
}

const ratio = (swatch: Swatch, scheme: Scheme): string => `${swatch.contrast[scheme].ratio.toFixed(2)}:1`

const useSummary = (swatch: Swatch): string => {
  const tokens = swatch.uses.map(use => use.token)
  return tokens.length === 0 ? 'none' : `${tokens.length} · ${tokens.slice(0, 2).join(', ')}${tokens.length > 2 ? ', …' : ''}`
}

const levelWord = (swatch: Swatch, scheme: Scheme) => {
  const { level } = swatch.contrast[scheme]
  return (
    <>
      <span aria-hidden="true" class={['font-display text-1-02rem font-400 whitespace-nowrap italic', LEVEL_INK[level]]}>{level}</span>
      <span class="sr-only">{`, ${LEVEL_TEXT[level]}`}</span>
    </>
  )
}

function values(swatch: Swatch) {
  return (
    <dl class="identity-values">
      <div><dt class="kicker">Hex</dt><dd><IdentityCopy label={`${swatch.title}’s hex, ${swatch.hex}`} value={swatch.hex} /></dd></div>
      <div><dt class="kicker">Token</dt><dd><IdentityCopy label={`${swatch.title}’s token, ${swatch.token}`} value={swatch.token} /></dd></div>
      <div><dt class="kicker">OKLCH</dt><dd class="font-mono text-xs text-text-1 tabular-nums">{swatch.oklch}</dd></div>
      {swatch.mix && (
        <div>
          <dt class="kicker">Mix</dt>
          <dd class="text-xs text-text-2">{`${swatch.mix.share}% toward ${swatch.mix.toward} from ${lead.title}`}</dd>
        </div>
      )}
    </dl>
  )
}

function contrast(swatch: Swatch) {
  return (
    <dl class="identity-values">
      {SCHEME_ORDER.map(scheme => (
        <div key={scheme}>
          <dt class="kicker">{`On ${scheme}`}</dt>
          <dd>
            <span
              aria-hidden="true"
              class="identity-proof inline-grid h-26 w-34 place-items-center font-display text-1rem leading-none font-500 italic"
              style={{ background: data.page[scheme], color: swatch.hex }}
            >Aa</span>
            <IdentityCopy label={`${swatch.title}’s contrast on ${scheme}, ${ratio(swatch, scheme)}`} value={ratio(swatch, scheme)} variant="plain" />
            {levelWord(swatch, scheme)}
          </dd>
        </div>
      ))}
    </dl>
  )
}

function uses(swatch: Swatch) {
  if (swatch.uses.length === 0) return <p class="m-0 text-xs text-text-3 italic">No family, role, or section takes it.</p>
  return (
    <table class="identity-table identity-uses text-xs">
      <thead><tr><th class="kicker">Token</th><th class="kicker">Where</th><th class="kicker">Scheme</th></tr></thead>
      <tbody>
        {swatch.uses.map(use => (
          <tr key={`${use.token} ${use.where} ${use.schemes.join()}`}>
            <td class="font-mono whitespace-nowrap text-text-1 max-mobile:break-all max-mobile:whitespace-normal">{use.token}</td>
            <td class="leading-1-4 break-all text-text-2">{use.where}</td>
            <td class="font-mono whitespace-nowrap text-text-1">{SCHEME_WORD(use.schemes)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  )
}

const plain = (swatch: Swatch): string => [
  `${groups.find(group => group.id === swatch.group)!.title.toLowerCase()}`,
  swatch.hex,
  swatch.token,
  `OKLCH ${swatch.oklch}`,
  ...SCHEME_ORDER.map(scheme => `on ${data.page[scheme]} ${ratio(swatch, scheme)}, ${LEVEL_TEXT[swatch.contrast[scheme].level]}`),
  ...swatch.mix ? [`${swatch.mix.share}% toward ${swatch.mix.toward} from ${lead.title}`] : [],
  ...swatch.uses.map(use => `${use.token} in ${use.where}, ${SCHEME_WORD(use.schemes)}`)
].join(' · ')

const [[text, textFloor], [mark, markFloor]] = LEVELS

// Sets out every color the palette declares as one band of strips grouped by
// the declaration that assigns each one, the color the reader points at,
// focuses, or pins named in the entry under the band with its values, its
// contrast on both page grounds, and every place the site reads it.
export const IdentityPalette = defineComponent({
  name : 'IdentityPalette',
  setup() {
    const { on, pinned, shown } = useRaised<string>()
    const band   = shallowRef<HTMLElement | null>(null)
    const roving = computed(() => pinned.value ?? lead.name)
    const held   = shallowRef(lead.name)

    const { start: letGo, stop: hold } = useTimeoutFn(() => { held.value = shown.value ?? lead.name }, GRACE_MS, { immediate: false })

    watch(shown, name => {
      if (name) {
        hold()
        held.value = name
      } else {
        letGo()
      }
    })

    const swatch = computed(() => data.swatches.find(each => each.name === held.value)!)

    function move(event: KeyboardEvent, at: number): void {
      const to = arrowIndex(event.key, at, strips.length, true)
      if (to === undefined) return
      event.preventDefault()
      band.value!.querySelectorAll<HTMLElement>('.identity-strip')[to]!.focus()
    }

    return () => (
      <div>
        <IdentityPanel source="palette.ts, by declaration" title="Paint band">
          {{
            aside   : () => <span class="kicker">{pinned.value ? `Pinned · ${data.swatches.find(each => each.name === pinned.value)!.title}` : ''}</span>,
            default : () => (
              <>
                <div aria-label="The palette as one band" class="flex gap-8 px-16 pt-16 pb-12 max-mobile:gap-5 max-mobile:px-10" ref={band} role="radiogroup">
                  {groups.map(group => (
                    <div class="grid min-w-min gap-5" key={group.id} style={{ flex: `${group.strips.length + (group.id === 'brand' ? 1 : 0)} 1 0` }}>
                      <span class="font-mono text-kicker-sm tracking-kicker whitespace-nowrap uppercase">
                        <span class="max-mobile:hidden">{group.title}</span>
                        <span class="hidden max-mobile:inline">{group.numeral}</span>
                      </span>
                      <div class="flex h-76 gap-2">
                        {group.strips.map(each => (
                          <button
                            aria-checked={each.name === held.value}
                            aria-label={`${each.title}, ${each.hex}`}
                            class="identity-strip"
                            data-lead={each === lead || undefined}
                            key={each.name}
                            onKeydown={(event: KeyboardEvent) => move(event, strips.indexOf(each))}
                            role="radio"
                            style={{ '--chip': `var(${each.token})` }}
                            tabindex={each.name === roving.value ? 0 : -1}
                            type="button"
                            {...on(each.name)}
                          />
                        ))}
                      </div>
                    </div>
                  ))}
                </div>
                <div
                  class="identity-entry border-t-1 border-t-divider px-16 pt-14 pb-16"
                  onMouseenter={hold}
                  onMouseleave={() => { if (!shown.value) letGo() }}
                >
                  <div class="identity-entry-head items-center gap-x-12 pb-12">
                    <span aria-hidden="true" class="identity-proof block h-44 w-full" style={{ background: `var(${swatch.value.token})` }} />
                    <div>
                      <p class="m-0 font-display text-1-4rem leading-1-1 font-500 text-text-1 italic">{swatch.value.title}</p>
                      <p class="mt-4 mb-0 kicker">{meta(swatch.value)}</p>
                    </div>
                  </div>
                  <IdentityRow key={`values ${swatch.value.name}`} label="Values" open summary={`${swatch.value.hex} · ${swatch.value.token}`}>
                    {{ default: () => values(swatch.value) }}
                  </IdentityRow>
                  <IdentityRow
                    key={`contrast ${swatch.value.name}`}
                    label="Contrast"
                    summary={SCHEME_ORDER.map(scheme => `${ratio(swatch.value, scheme)} ${swatch.value.contrast[scheme].level} on ${scheme}`).join(' · ')}
                  >
                    {{ default: () => contrast(swatch.value) }}
                  </IdentityRow>
                  <IdentityRow key={`uses ${swatch.value.name}`} label="Uses" summary={useSummary(swatch.value)}>
                    {{ default: () => uses(swatch.value) }}
                  </IdentityRow>
                </div>
                <p aria-live="polite" class="sr-only">{pinned.value ? `${swatch.value.title} pinned, ${swatch.value.hex}` : ''}</p>
              </>
            )
          }}
        </IdentityPanel>
        <IdentityCaption>
          {{
            default: () => [
              'The band sets every color as one strip, grouped by declaration, and the entry under it follows the strip a reader points at or focuses, a click pinning it until Escape. Each ratio sets a color against the page ground, ',
              <code>{data.page.light}</code>,
              ' in the light scheme and ',
              <code>{data.page.dark}</code>,
              ` in the dark, and the word that follows it names what the ratio passes, ${text} at ${textFloor}:1 or more and ${mark} at ${markFloor}:1 or more, or fails under ${markFloor}:1.`
            ]
          }}
        </IdentityCaption>
        <IdentityPlain label="Every value as plain text">
          {{
            default: () => (
              <ol class="m-0 list-none p-0">
                {data.swatches.map(each => (
                  <li class="mt-5 first:mt-0" key={each.name}>
                    <strong class="font-600 text-text-1">{each.title}</strong>
                    {` · ${plain(each)}`}
                  </li>
                ))}
              </ol>
            )
          }}
        </IdentityPlain>
      </div>
    )
  }
})
