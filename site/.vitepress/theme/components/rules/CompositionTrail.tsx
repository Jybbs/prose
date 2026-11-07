import { useData }                        from 'vitepress'
import { computed, defineComponent, ref } from 'vue'

import { NibBadge, Tabs } from '../../../kit'
import type { TabsScope } from '../../../kit'

import { COMPOSITION_RULE, fixtureEntry, fixtureTrail } from '../../../lib/fixtures/entry'
import { changeValues }                               from '../../../lib/fixtures/gauge'
import { data as rules }                              from '../../../lib/rules/rules.data'
import { familyEmblem }                               from '../../../lib/shared/emblems'
import { FixtureLead }                                from '../fixtures/FixtureLead'

const WRITTEN = '0'

// Lays a composition case's rules out as one trail in pipeline order, each step
// showing the case formatted with `--select` naming the rules up to it.
export const CompositionTrail = defineComponent({
  name  : 'CompositionTrail',
  props : {
    case : { required: true, type: String }
  },
  setup(props) {
    const { frontmatter } = useData()

    const entry = computed(() => fixtureEntry(frontmatter.value, COMPOSITION_RULE, props.case))
    const trail = computed(() => fixtureTrail(frontmatter.value, props.case))
    const step  = ref(String(trail.value.rules.length))

    const changes = computed(() => new Set(changeValues(trail.value.steps)))
    const states  = computed(() => trail.value.steps.map(at => trail.value.outputs[at]!))
    const through = computed(() => Number(step.value))

    return () => (
      <section class="my-1-25rem">
        <FixtureLead nodes={entry.value.descriptionNodes} />
        <Tabs label="Rules in pipeline order" modelValue={step.value} onUpdate:modelValue={next => { step.value = next }}>
          {{
            default: (tabs: TabsScope<string>) => (
              <div {...tabs.list} class="mb-12 toggle-row flex-wrap items-center gap-2">
                <button {...tabs.trigger(WRITTEN)} aria-controls={undefined} class="composition-trail-step toggle-pill" type="button">
                  As written
                </button>
                {trail.value.rules.map((slug, at) => {
                  const family = rules.bySlug[slug]?.family
                  return (
                    <button
                      key={slug}
                      {...tabs.trigger(String(at + 1))}
                      aria-controls={undefined}
                      class="composition-trail-step toggle-pill inline-flex items-center gap-4"
                      data-family={family}
                      data-still={!changes.value.has(at + 1) || undefined}
                      type="button"
                    >
                      {family ? <NibBadge class="text-family-nib-chip" emblem={familyEmblem(family)} size={13} /> : null}
                      {slug}
                    </button>
                  )
                })}
              </div>
            )
          }}
        </Tabs>
        <p class="my-0 mb-8 font-mono text-meta text-text-2">
          {through.value === 0
            ? 'The case as written, before any rule runs'
            : <code>{`prose format --select ${trail.value.rules.slice(0, through.value).join(',')}`}</code>}
        </p>
        <div class="grid overflow-hidden panel bg-code-block">
          {states.value.map((html, at) => (
            <div
              class="col-start-1 row-start-1 min-w-0 inert:invisible *:m-0"
              inert={at !== through.value}
              innerHTML={html}
              key={at}
            />
          ))}
        </div>
      </section>
    )
  }
})
