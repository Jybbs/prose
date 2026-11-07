import { computed, defineComponent, ref, watch } from 'vue'

import { nib, NibBadge, Tabs }                from '../../../kit'
import type { TabsScope }                     from '../../../kit'
import { useReducedMotion }                   from '../../../lib/composables/use-reduced-motion'
import { data }                               from '../../../lib/identity/identity.data'
import { familyEmblem, strokesOf }            from '../../../lib/shared/emblems'
import { FAMILY_ORDER, GLOSSARY_FAMILY_META } from '../../../lib/shared/registries'
import { IdentityCopyChip, IdentityRow, IdentitySchemeLabel } from './IdentityParts'

import type { GlossaryFamily, RuleFamily } from '../../../lib/shared/registries'

const GROUPS = [
  { families: FAMILY_ORDER as readonly GlossaryFamily[], title: 'Rule families' },
  {
    families : (Object.keys(GLOSSARY_FAMILY_META) as GlossaryFamily[]).filter(family => !(FAMILY_ORDER as readonly string[]).includes(family)),
    title    : 'Cross-cutting concerns'
  }
]

// Draws one family's badge large, stroke by stroke in the order its motif
// declares them, beside a list of every family grouped as rule families and
// cross-cutting concerns, and reads out the ink the badge takes, its strokes,
// and the rules the family holds.
export const IdentityBadges = defineComponent({
  name : 'IdentityBadges',
  setup() {
    const chosen  = ref<GlossaryFamily>(FAMILY_ORDER[0])
    const drawn   = ref(0)
    const reduced = useReducedMotion()
    const emblem  = computed(() => familyEmblem(chosen.value))
    const strokes = computed(() => strokesOf(emblem.value.motif))
    const shown   = computed(() => reduced.value ? strokes.value.length : drawn.value)

    watch(chosen, () => { drawn.value = 0 })

    return () => {
      const { fit, label } = emblem.value.motif
      return (
        <Tabs label="Families" modelValue={chosen.value} onUpdate:modelValue={(next: GlossaryFamily) => { chosen.value = next }} orientation="vertical">
          {{
            default: ({ content, list, trigger }: TabsScope<GlossaryFamily>) => (
              <div class="my-1-5rem grid grid-cols-196-minmax-1fr gap-1-5rem max-tablet:grid-cols-1fr">
                <div {...list} class="grid content-start">
                  {GROUPS.map(group => [
                    <span aria-hidden="true" class="kicker mt-0-5rem" key={group.title}>{group.title}</span>,
                    ...group.families.map(family => (
                      <button
                        key={family}
                        {...trigger(family)}
                        aria-controls={undefined}
                        class="identity-legend flex items-center gap-x-8 text-sm text-text-1"
                        data-family={family}
                      >
                        <NibBadge emblem={familyEmblem(family)} size={16} />
                        {GLOSSARY_FAMILY_META[family].label}
                      </button>
                    ))
                  ])}
                </div>
                <div {...content(chosen.value)} class="grid content-start gap-1rem" data-family={chosen.value}>
                  <div class="grid place-items-center rounded-sm border-1 border-divider bg-bg p-1-25rem">
                    <svg
                      aria-label={`${GLOSSARY_FAMILY_META[chosen.value].label} badge, the ${label.toLowerCase()}`}
                      class="identity-badge size-9rem"
                      key={chosen.value}
                      role="img"
                      viewBox="0 0 24 24"
                    >
                      <g filter={`url(#${nib.nibId(emblem.value.motif, 'jitter')})`}>
                        <g transform={nib.fitTransform(fit)}>
                          <g {...nib.STROKE} stroke-width={nib.perScale(fit, nib.NIB.stroke.nib)}>
                            {nib.NIB.large.map(offset => (
                              <g key={offset} transform={nib.diagonal(fit, offset)}>
                                {strokes.value.map((d, index) => (
                                  <path
                                    class="identity-stroke"
                                    d={d}
                                    key={d}
                                    onAnimationend={offset === 0 ? () => { drawn.value = index + 1 } : undefined}
                                    pathLength={1}
                                    style={{ '--stroke': index }}
                                  />
                                ))}
                              </g>
                            ))}
                          </g>
                        </g>
                      </g>
                    </svg>
                    <p aria-live="polite" class="m-0 font-mono text-meta text-text-3 tabular-nums">
                      {`Stroke ${shown.value} of ${strokes.value.length}`}
                    </p>
                  </div>
                  <div class="grid grid-cols-max-1fr gap-x-1-5rem">
                    <IdentityRow label="Ink">{{ default: () => (
                      <div class="grid gap-6">
                        {data.schemes.map(scheme => (
                          <div class="flex flex-wrap items-center gap-8" key={scheme}>
                            <IdentitySchemeLabel scheme={scheme} />
                            <IdentityCopyChip label={`the ${scheme} ink`} value={data.inks[chosen.value][scheme]} />
                          </div>
                        ))}
                        <div><IdentityCopyChip label="the ink token" value="--family-nib" /></div>
                      </div>
                    ) }}</IdentityRow>
                    <IdentityRow label="Strokes">{{ default: () => (
                      <p class="m-0 text-sm tabular-nums">{strokes.value.length}</p>
                    ) }}</IdentityRow>
                    <IdentityRow label="Rules">{{ default: () => (
                      <p class="m-0 text-sm tabular-nums">
                        {chosen.value in data.ruleCounts ? data.ruleCounts[chosen.value as RuleFamily] : 'A concern rather than a rule family'}
                      </p>
                    ) }}</IdentityRow>
                  </div>
                </div>
              </div>
            )
          }}
        </Tabs>
      )
    }
  }
})
