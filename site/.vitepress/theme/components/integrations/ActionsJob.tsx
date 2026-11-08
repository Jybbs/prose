import { computed, defineComponent, ref } from 'vue'

import { CopyButton, Tabs } from '../../../kit'
import type { TabsScope }   from '../../../kit'

import * as job            from '../../../lib/integrations/actions-job'
import { data as actions } from '../../../lib/integrations/actions-job.data'

const ADDED = ['cache', 'ruff'] as const

const FULLEST = actions.workflows[job.choiceKey({ cache: true, format: job.UPLOADED_FORMAT, ruff: true })]!

// Draws the `prose check` command a job runs, its format tokens choosing the
// output format and its `+ cache` and `+ ruff` tokens adding their steps to the
// workflow beneath, whose panel holds the height of the workflow every step and
// the upload add up to.
export const ActionsJob = defineComponent({
  name : 'ActionsJob',
  setup() {
    const choice = ref<job.JobChoice>({ cache: false, format: job.DEFAULT_FORMAT, ruff: false })
    const html   = computed(() => actions.workflows[job.choiceKey(choice.value)]!)
    const yaml   = computed(() => job.jobYaml(choice.value))

    return () => (
      <figure class="my-1-25rem">
        <figcaption class="mb-8 flex flex-wrap items-center gap-x-8 gap-y-4 font-mono text-meta text-text-2">
          <span class="text-text-1">prose check</span>
          <Tabs
            label="Output format"
            modelValue={choice.value.format}
            onUpdate:modelValue={format => { choice.value = { ...choice.value, format } }}
          >
            {{
              default: (tabs: TabsScope<string>) => (
                <span {...tabs.list} class="toggle-row flex-wrap">
                  {actions.formats.map(format => (
                    <button
                      key={format}
                      {...tabs.trigger(format)}
                      aria-controls={undefined}
                      class="code-pill border-none"
                      type="button"
                    >{format === job.DEFAULT_FORMAT ? format : `--output-format ${format}`}</button>
                  ))}
                </span>
              )
            }}
          </Tabs>
          <span aria-hidden="true">.</span>
          {ADDED.map(step => (
            <button
              aria-pressed={choice.value[step]}
              class="code-pill border-1 border-divider aria-pressed:border-text-2"
              key={step}
              onClick={() => { choice.value = { ...choice.value, [step]: !choice.value[step] } }}
              type="button"
            >{`+ ${step}`}</button>
          ))}
        </figcaption>
        <div class="copy-host relative grid overflow-hidden panel bg-code-block">
          <div aria-hidden="true" class="invisible col-start-1 row-start-1 min-w-0 flush-code-block" inert innerHTML={FULLEST} />
          <div class="col-start-1 row-start-1 min-w-0 flush-code-block" innerHTML={html.value} />
          <CopyButton label="Copy workflow" source={yaml.value} />
        </div>
      </figure>
    )
  }
})
