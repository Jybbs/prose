import { computed, defineComponent } from 'vue'

import { useFixtureEntry } from '../../../lib/composables/use-fixture-entry'
import { InlineProse }     from '../base/InlineProse'

interface Step {
  badge ?: 'changed' | 'settled'
  html  ?: string
  label  : string
}

export const FixtureConvergence = defineComponent({
  name  : 'FixtureConvergence',
  props : {
    case : { required: true, type: String },
    rule : { required: true, type: String }
  },
  setup(props) {
    const entry = useFixtureEntry(props)

    // Lists the input as written ahead of each run over it. A fixture that
    // rewrites its input settles on the run after the one that changed it, whereas
    // a fixed-point fixture is already settled on run one.
    const steps = computed<Step[]>(() => {
      const written: Step = { html: entry.value.inputHtml, label: 'As written' }
      return entry.value.changesSource
        ? [
            written,
            { badge: 'changed', html: entry.value.outputHtml, label: 'Run 1' },
            { badge: 'settled', label: 'Run 2' }
          ]
        : [written, { badge: 'settled', label: 'Run 1' }]
    })

    return () => (
      <section class="my-1-25rem">
        {entry.value.descriptionNodes && (
          <div class="fixture-lead mb-1rem">
            <InlineProse nodes={entry.value.descriptionNodes} />
          </div>
        )}
        <ol class="flex flex-col gap-10 border-l-1 border-l-divider">
          {steps.value.map(step => (
            <li
              class="fixture-converge-step group relative before:absolute before:top-0-45rem before:size-7 before:rounded-half before:border-1 before:border-divider before:bg-bg data-[badge=settled]:before:border-celadon data-[badge=settled]:before:bg-celadon"
              data-badge={step.badge}
              key={step.label}
            >
              <p class="flex items-baseline gap-10">
                <span class="font-mono text-meta tracking-label-loose text-text-1 uppercase">{step.label}</span>
                {step.badge && <span class="change-chip px-8 py-2 group-data-[badge=settled]:border-celadon/40 group-data-[badge=settled]:text-celadon-soft-45">{step.badge === 'changed' ? 'Rewritten' : 'No change'}</span>}
              </p>
              {step.html
                ? (
                  <div
                    class="fixture-converge-state overflow-hidden panel bg-code-block"
                    innerHTML={step.html}
                  />
                )
                : <p class="fixture-converge-note text-meta text-text-2"> This run reads the previous output and rewrites nothing, so the file has reached its fixed point and every later run leaves it exactly as it stands. </p>}
            </li>
          ))}
        </ol>
      </section>
    )
  }
})
