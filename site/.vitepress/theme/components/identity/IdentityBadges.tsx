import { computed, defineComponent, shallowRef, watch } from 'vue'

import { nib, NibBadge, Tabs }                from '../../../kit'
import type { TabsScope }                     from '../../../kit'
import { useReducedMotion }                   from '../../../lib/composables/use-reduced-motion'
import { data }                               from '../../../lib/identity/identity.data'
import { familyEmblem, strokesOf }            from '../../../lib/shared/emblems'
import { counted }                            from '../../../lib/shared/numerals'
import { FAMILY_ORDER, GLOSSARY_FAMILY_META } from '../../../lib/shared/registries'
import { IdentityCaption, IdentityPanel, IdentityPlain } from './IdentityParts'

import type { GlossaryFamily, RuleFamily } from '../../../lib/shared/registries'

const GROUPS = [
  { families: FAMILY_ORDER as readonly GlossaryFamily[], title: 'Rule families' },
  {
    families : (Object.keys(GLOSSARY_FAMILY_META) as GlossaryFamily[]).filter(family => !(FAMILY_ORDER as readonly string[]).includes(family)),
    title    : 'Cross-cutting concerns'
  }
]

const FAMILIES = GROUPS.flatMap(group => group.families)

// Draws the large badge this many pixels square.
const BADGE_PX = 128

const rules = (family: GlossaryFamily): string =>
  family in data.ruleCounts ? counted(data.ruleCounts[family as RuleFamily], 'rule') : 'a concern'

const holds = (family: GlossaryFamily): string => family in data.ruleCounts
  ? `The family holds ${rules(family)}.`
  : 'It marks a concern rather than a rule family.'

const sentence = (family: GlossaryFamily): string => {
  const { badge, label } = GLOSSARY_FAMILY_META[family]
  const strokes          = strokesOf(familyEmblem(family).motif).length
  const ink              = data.inks[family]
  return `The ${label.toLowerCase()} badge is a ${familyEmblem(family).motif.label.toLowerCase()} ${badge} drawn in ${counted(strokes, 'stroke')}, `
    + `inked ${ink.light} on the light page and ${ink.dark} on the dark, the family color shaded to ${data.inkRatio}:1 against each.`
}

// Draws one family's badge large, stroke by stroke in the order its motif
// declares them, beside the list of every family grouped as rule families and
// cross-cutting concerns, and reads out the badge's motif, its ink, its
// strokes, and the rules the family holds.
export const IdentityBadges = defineComponent({
  name : 'IdentityBadges',
  setup() {
    const chosen  = shallowRef<GlossaryFamily>(FAMILY_ORDER[0])
    const drawn   = shallowRef(0)
    const reduced = useReducedMotion()
    const emblem  = computed(() => familyEmblem(chosen.value))
    const strokes = computed(() => strokesOf(emblem.value.motif))
    const shown   = computed(() => reduced.value ? strokes.value.length : drawn.value)

    watch(chosen, () => { drawn.value = 0 })

    return () => {
      const { fit, label } = emblem.value.motif
      const meta           = GLOSSARY_FAMILY_META[chosen.value]
      return (
        <div>
          <IdentityPanel source="NibSprite" title="One drawing">
            {{
              aside   : () => <span class="kicker">{`${BADGE_PX} px`}</span>,
              default : () => (
                <Tabs label="Families" modelValue={chosen.value} onUpdate:modelValue={(next: GlossaryFamily) => { chosen.value = next }} orientation="vertical">
                  {{
                    default: ({ content, list, trigger }: TabsScope<GlossaryFamily>) => (
                      <div class="px-18 pb-16">
                        <div class="identity-badges">
                          <div {...content(chosen.value)} class="grid place-content-center justify-items-center gap-10 p-16" data-family={chosen.value}>
                            <svg
                              aria-label={`${meta.label} badge, the ${label.toLowerCase()}`}
                              class="identity-badge overflow-visible"
                              height={BADGE_PX}
                              key={chosen.value}
                              role="img"
                              viewBox="0 0 24 24"
                              width={BADGE_PX}
                            >
                              <g filter={`url(#${nib.nibId(emblem.value.motif, 'jitter')})`}>
                                {nib.fitted(fit, nib.NIB.stroke.nib, nib.NIB.large.map(offset => (
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
                                )))}
                              </g>
                            </svg>
                            <p aria-live="polite" class="m-0 kicker tabular-nums">{`Stroke ${shown.value} of ${strokes.value.length}`}</p>
                          </div>
                          <div {...list} class="identity-side grid content-start py-8">
                            {GROUPS.map(group => [
                              <span aria-hidden="true" class="mx-14 mt-8 mb-4 block kicker" key={group.title}>{group.title}</span>,
                              ...group.families.map(family => (
                                <button key={family} {...trigger(family)} aria-controls={undefined} class="identity-pick identity-badge-row" data-family={family}>
                                  <NibBadge emblem={familyEmblem(family)} size={18} />
                                  <span class="identity-pick-name font-display text-0-98rem font-500 text-text-2 italic">{GLOSSARY_FAMILY_META[family].label}</span>
                                  <span aria-hidden="true" class="text-0-84rem">{GLOSSARY_FAMILY_META[family].badge}</span>
                                </button>
                              ))
                            ])}
                          </div>
                        </div>
                        <p class="mt-0 mb-0 flex flex-wrap items-baseline gap-x-8 gap-y-4 border-t-1 border-t-divider pt-12 text-xs">
                          <span class="font-display text-1rem font-500 text-text-1 italic">{meta.label}</span>
                          <span aria-hidden="true" class="middle-dot">·</span>
                          <span class="kicker">{`${meta.badge} ${label}`}</span>
                          <span aria-hidden="true" class="middle-dot">·</span>
                          <span class="font-mono text-text-1 tabular-nums">
                            {'ink '}
                            <span class="dark:hidden">{data.inks[chosen.value].light}</span>
                            <span class="hidden dark:inline">{data.inks[chosen.value].dark}</span>
                          </span>
                          <span aria-hidden="true" class="middle-dot">·</span>
                          <span class="font-mono text-text-1 tabular-nums">{counted(strokes.value.length, 'stroke')}</span>
                          <span aria-hidden="true" class="middle-dot">·</span>
                          <span class="font-mono text-text-1 tabular-nums">{rules(chosen.value)}</span>
                        </p>
                        <IdentityPlain label="Every badge’s caption, as text">
                          {{
                            default: () => (
                              <ol class="m-0 list-none p-0">
                                {FAMILIES.map(family => <li class="mt-5 first:mt-0" key={family}>{`${sentence(family)} ${holds(family)}`}</li>)}
                              </ol>
                            )
                          }}
                        </IdentityPlain>
                      </div>
                    )
                  }}
                </Tabs>
              )
            }}
          </IdentityPanel>
          <IdentityCaption>{{ default: () => sentence(chosen.value) }}</IdentityCaption>
        </div>
      )
    }
  }
})
