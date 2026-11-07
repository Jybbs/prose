import { computed, defineComponent, ref } from 'vue'

import { Kicker, ScrollRegion, Tabs } from '../../../kit'
import type { TabsScope }             from '../../../kit'
import { data }                       from '../../../lib/identity/identity.data'
import { SCHEME_TEXT }                from '../../../lib/identity/roles'
import { formatFolio }                from '../../../lib/shared/numerals'
import { IdentityCopyChip, IdentityRow, IdentitySchemeLabel } from './IdentityParts'

import type { Level, Swatch } from '../../../lib/identity/swatches'
import type { Scheme }        from '../../../lib/shared/palette'

const LEVEL_TEXT: Record<Level, string> = {
  fails : 'Fails as text and as a mark',
  mark  : 'Passes as a mark',
  text  : 'Passes as text'
}

const { schemes: SCHEMES } = data

const groups = data.groups.map(group => ({ ...group, swatches: data.swatches.filter(swatch => swatch.group === group.id) }))

const schemesOf = (swatch: Swatch): readonly Scheme[] => swatch.values.dark === swatch.values.light ? ['light'] : SCHEMES

const schemeText = (schemes: readonly Scheme[]): string =>
  schemes.length === SCHEMES.length ? 'Both' : schemes.map(scheme => SCHEME_TEXT[scheme]).join(', ')

function values(swatch: Swatch) {
  const schemes = schemesOf(swatch)
  return (
    <div class="grid gap-y-6">
      {schemes.map(scheme => (
        <div class="flex flex-wrap items-center gap-x-8 gap-y-4" key={scheme}>
          {schemes.length > 1 && <IdentitySchemeLabel scheme={scheme} />}
          <IdentityCopyChip label={`the ${SCHEME_TEXT[scheme].toLowerCase()} hex`} value={swatch.values[scheme]} />
          <span class="font-mono text-xs text-text-2">{swatch.oklch[scheme]}</span>
        </div>
      ))}
      <div><IdentityCopyChip label="the token" value={swatch.token} /></div>
      {swatch.mix && (
        <p class="m-0 text-sm text-text-2">
          {`${data.brand} mixed ${swatch.mix.share}% toward ${swatch.mix.toward} in OKLCH.`}
        </p>
      )}
    </div>
  )
}

function contrast(swatch: Swatch) {
  return (
    <div class="flex flex-wrap gap-x-1-5rem gap-y-8">
      {SCHEMES.map(scheme => (
        <div class="flex items-center gap-10" key={scheme}>
          <span
            aria-hidden="true"
            class="grid size-3rem place-items-center rounded-sm border-1 border-divider font-display text-1-5rem"
            style={{ background: data.roles.page[scheme], color: swatch.values[scheme] }}
          >Aa</span>
          <span class="grid">
            <span class="font-mono text-sm text-text-1 tabular-nums">{`${swatch.contrast[scheme].ratio.toFixed(2)}:1`}</span>
            <span class="text-xs text-text-2">{`${SCHEME_TEXT[scheme]} page · ${LEVEL_TEXT[swatch.contrast[scheme].level]}`}</span>
          </span>
        </div>
      ))}
    </div>
  )
}

function uses(swatch: Swatch) {
  if (swatch.uses.length === 0) return <p class="m-0 text-sm text-text-2">The site reads this color nowhere outside its declaration.</p>
  return (
    <ScrollRegion class="max-h-30rem overflow-auto">{{ default: () => (
      <table class="identity-table m-0 w-full text-xs max-tablet:whitespace-nowrap">
        <thead><tr><th>Token</th><th>Where</th><th>Scheme</th></tr></thead>
        <tbody>
          {swatch.uses.map(use => (
            <tr key={`${use.token} ${use.where} ${use.schemes.join()}`}>
              <td><code>{use.token}</code></td>
              <td class="break-all">{use.where}</td>
              <td>{schemeText(use.schemes)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    ) }}</ScrollRegion>
  )
}

// Sets out every color the palette declares as a band of strips grouped by the
// declaration that assigns each one, the chosen color named beneath it with its
// values, its contrast on both page grounds, and every place the site reads it.
export const IdentityPalette = defineComponent({
  name : 'IdentityPalette',
  setup() {
    const chosen = ref(data.swatches[0]!.name)
    const swatch = computed(() => data.swatches.find(each => each.name === chosen.value)!)
    const folio  = computed(() => {
      const group = groups.find(each => each.id === swatch.value.group)!
      return `${group.title} · ${formatFolio(group.swatches.indexOf(swatch.value) + 1)}`
    })

    return () => (
      <Tabs label="Palette colors" modelValue={chosen.value} onUpdate:modelValue={(name: string) => { chosen.value = name }}>
        {{
          default: ({ content, list, trigger }: TabsScope<string>) => (
            <div class="my-1-5rem">
              <div {...list} class="flex flex-wrap gap-x-6 gap-y-0-75rem">
                {groups.map(group => (
                  <div class="grid min-w-0 gap-y-4" key={group.id} style={{ flexGrow: group.swatches.length }}>
                    <div class="flex h-3rem">
                      {group.swatches.map(each => (
                        <button
                          key={each.name}
                          {...trigger(each.name)}
                          aria-controls={undefined}
                          aria-label={each.title}
                          class="identity-strip min-w-0 flex-1 cursor-pointer border-none p-0 focus-visible:focus-ring focus-visible:outline-offset-1"
                          style={{ background: `var(${each.token})` }}
                          title={each.title}
                        />
                      ))}
                    </div>
                    <span aria-hidden="true" class="kicker">{group.title}</span>
                  </div>
                ))}
              </div>
              <div {...content(chosen.value)} class="mt-1-25rem grid grid-cols-max-1fr gap-x-1-5rem">
                <div class="col-span-full pb-0-75rem">
                  <Kicker class="m-0">{{ default: () => folio.value }}</Kicker>
                  <p class="m-0 font-display text-2rem leading-1-2 italic text-text-1">{swatch.value.title}</p>
                </div>
                <IdentityRow label="Values">{{ default: () => values(swatch.value) }}</IdentityRow>
                <IdentityRow label="Contrast">{{ default: () => contrast(swatch.value) }}</IdentityRow>
                <IdentityRow label="Uses">{{ default: () => uses(swatch.value) }}</IdentityRow>
              </div>
            </div>
          )
        }}
      </Tabs>
    )
  }
})
