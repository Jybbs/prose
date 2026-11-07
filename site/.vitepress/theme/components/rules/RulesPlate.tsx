import { computed, defineComponent, nextTick, shallowRef, watch } from 'vue'

import { NibBadge, paint } from '../../../kit'

import { useRaisedFamily } from '../../../lib/composables/use-raised-family'
import { glide }           from '../../../lib/rules/plate-glide'
import { data as rules }   from '../../../lib/rules/rules.data'
import { arrowIndex }      from '../../../lib/shared/arrow-keys'
import { familyEmblem }    from '../../../lib/shared/emblems'
import { FAMILY_ORDER }    from '../../../lib/shared/registries'
import type { RuleFamily } from '../../../lib/shared/registries'

const KEY = FAMILY_ORDER.map(family => ({
  count  : rules.list.filter(rule => rule.family === family).length,
  emblem : familyEmblem(family),
  family : family
}))

export const RulesPlate = defineComponent({
  name: 'RulesPlate',
  setup() {
    const plate = shallowRef<HTMLElement | null>(null)

    const { on, pinned, release, shown } = useRaisedFamily()

    // Holds the family whose rules the list shows gathered above the letters,
    // which follows the pinned family once each move has measured the list.
    const arranged = shallowRef<RuleFamily | null>(null)
    const roving   = shallowRef<RuleFamily>(FAMILY_ORDER[0])
    const spoken   = shallowRef('')

    const counted = computed(() => KEY.find(({ family }) => family === shown.value))

    // Lists the group of the arranged family's rules above the letters, each
    // letter holding the rest of its rules.
    const sections = computed(() => {
      const held    = arranged.value
      const letters = [...Map.groupBy(rules.list, rule => rule.slug[0]!)].map(([key, group]) =>
        ({ family: null, key, rules: group.filter(rule => rule.family !== held) }))
      if (!held) return letters
      return [{ family: held, key: 'gathered', rules: rules.list.filter(rule => rule.family === held) }, ...letters]
    })

    watch(pinned, family => {
      const entry = KEY.find(each => each.family === family)
      spoken.value = entry ? `${entry.count} ${entry.emblem.name} rules moved above the list.` : 'All families shown.'
      void glide(plate.value!, async () => {
        arranged.value = family as RuleFamily | null
        await nextTick()
      }, { duration: paint.ruleDrawMs(), easing: paint.morphEase() })
    })

    // Moves the focus and the key's one tab stop to the chip at `at`.
    function focusChip(at: number): void {
      roving.value = KEY[at]!.family
      plate.value!.querySelectorAll<HTMLElement>('.rules-plate-key button')[at]!.focus()
    }

    // Moves the focus across the key's chips under the arrow keys, Home, and
    // End, wrapping past either end.
    function move(event: KeyboardEvent, at: number): void {
      const to = arrowIndex(event.key, at, KEY.length, true)
      if (to === undefined) return
      event.preventDefault()
      focusChip(to)
    }

    // Returns every row to its letter and the focus to the chip that held them.
    function releaseAll(): void {
      const back = KEY.findIndex(({ family }) => family === pinned.value)
      release()
      focusChip(Math.max(back, 0))
    }

    return () => (
      <div class="relative isolate" ref={plate}>
        <div
          aria-label="Rule families"
          class="rules-plate-key mt-24 flex flex-wrap gap-x-4 gap-y-6 border-b-1 border-b-divider pb-20"
          role="toolbar"
        >
          {KEY.map(({ emblem, family }, at) => (
            <span class="rules-plate-split inline-flex items-stretch" data-family={family} key={family}>
              <button
                aria-pressed={family === pinned.value}
                tabindex={family === roving.value ? 0 : -1}
                type="button"
                {...on(family)}
                class="chip nib-ground cursor-pointer gap-6 pt-6 pr-4 pb-6 pl-8 font-mono text-label leading-24 font-400 tracking-label whitespace-nowrap text-text-1 uppercase hover:border-text-2 focus-visible:focus-ring focus-visible:outline-offset-2 aria-pressed:font-600"
                onClick={() => { roving.value = family }}
                onKeydown={event => move(event, at)}
              >
                <NibBadge emblem={emblem} size={15.2} />
                <span>{emblem.name}</span>
              </button>
              <a
                aria-label={`${emblem.name} page`}
                class="rules-plate-go inline-flex items-center border-1 border-l-0 border-divider bg-chip-ground px-6 py-0 font-mono text-xxs text-text-2 no-underline transition-colors hover:text-text-1 focus-visible:text-text-1"
                href={`/rules/${family}/`}
              >
                <span aria-hidden="true" class="nib-stroke is-quiet">→</span>
              </a>
            </span>
          ))}
        </div>
        <div class="mx-0 mt-14 mb-18 flex items-center gap-10">
          <div
            aria-hidden="true"
            class="rules-plate-census flex h-6 min-w-64 basis-1/3 items-end gap-2"
          >
            {KEY.map(({ count, family }) => (
              <span
                class={{ 'is-raised': family === shown.value }}
                data-family={family}
                key={family}
                style={{ flex: `${count} 1 0` }}
              />
            ))}
          </div>
          <span class="rules-plate-census-cap font-mono text-xs whitespace-nowrap text-text-2 tabular-nums">{counted.value ? `${counted.value.count} of ${rules.list.length}` : rules.list.length} rules </span>
          <button
            class={['rules-plate-all cursor-pointer rounded-sm border-1 border-divider bg-chip-ground px-6 py-0 font-mono text-xs leading-24 whitespace-nowrap text-text-2 transition-colors hover:border-text-2 hover:text-text-1 focus-visible:focus-ring focus-visible:outline-offset-2', { 'is-idle': !pinned.value }]}
            onClick={releaseAll}
            type="button"
          > All families </button>
        </div>
        <p aria-live="polite" class="rules-plate-live sr-only">{spoken.value}</p>
        {sections.value.map(section => (
          <section
            class={['rules-plate-letter grid-cols-3rem-minmax-1fr gap-x-14 gap-y-0 border-t-1 border-t-divider py-12', { 'is-out': !section.rules.length, 'rules-plate-gathered': section.family }]}
            data-letter={section.key}
            key={section.key}
          >
            <span
              aria-hidden="true"
              class="rules-plate-cap font-display text-2-1rem leading-none font-500 text-text-3 italic"
              data-family={section.family ?? undefined}
            >
              {section.family
                ? <NibBadge emblem={familyEmblem(section.family)} size={28} />
                : section.key.toUpperCase()}
            </span>
            <ul
              aria-label={section.family ? `${familyEmblem(section.family).name} rules` : undefined}
              class="rules-plate-slugs m-0 list-none columns-2-11rem gap-x-24 p-0"
            >
              {section.rules.map(rule => (
                <li
                  class="m-0 flex break-inside-avoid items-center gap-8 py-3 focus-within:relative focus-within:z-20 hover:relative hover:z-20"
                  data-family={rule.family}
                  data-slug={rule.slug}
                  key={rule.slug}
                >
                  <NibBadge emblem={familyEmblem(rule.family)} label={rule.familyLabel} size={14} />
                  <a
                    class="rules-plate-slug nib-stroke is-quiet pb-1 font-mono text-0-8rem leading-1-5 font-400 text-text-1 no-underline hover:text-brand-2 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-link"
                    data-rule={rule.slug}
                    href={rule.href}
                  >{rule.slug}</a>
                </li>
              ))}
            </ul>
          </section>
        ))}
      </div>
    )
  }
})
