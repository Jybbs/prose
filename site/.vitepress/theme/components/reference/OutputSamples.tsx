import { defineComponent, ref } from 'vue'

import { DiffSample } from './DiffSample'

import { Tabs }           from '../../../kit'
import type { TabsScope } from '../../../kit'

import { FORMATS, SAMPLE }   from '../../../lib/reference/output-samples'
import type { SampleFormat } from '../../../lib/reference/output-samples'
import { data }              from '../../../lib/reference/output-samples.data'

type Shown = SampleFormat | 'diff'

const SHOWN: readonly Shown[] = [...FORMATS, 'diff']

// Draws the sample input beside its output in each format, with an index of
// the findings by rule and position. Choosing a finding draws its squiggle on
// the input and marks its record in each format, and every panel shares one
// grid cell so a switch keeps the height of the tallest.
export const OutputSamples = defineComponent({
  name: 'OutputSamples',
  setup() {
    const shown  = ref<Shown>('text')
    const chosen = ref(0)

    const records = (format: SampleFormat) => data.records[format].map((html, at) => (
      <div
        class="border-l-2 border-transparent pl-8 transition-colors data-marked:border-apricot"
        data-marked={at === chosen.value || undefined}
        innerHTML={html}
        key={at}
      />
    ))

    return () => (
      <figure class="my-24">
        <figcaption class="code-caption">{SAMPLE.filename}</figcaption>
        <div class="overflow-hidden panel bg-code-block *:m-0" innerHTML={data.findings[chosen.value]?.inputHtml ?? data.inputHtml} />
        <ul aria-label="Findings by rule and position" class="my-12 flex list-none flex-wrap gap-6 p-0">
          {data.findings.map((finding, at) => (
            <li class="m-0" key={at}>
              <button
                aria-pressed={at === chosen.value}
                class="toggle-pill"
                onClick={() => { chosen.value = at }}
                type="button"
              >{`${finding.code} ${finding.row}:${finding.column}`}</button>
            </li>
          ))}
        </ul>
        <Tabs label="Output format" modelValue={shown.value} onUpdate:modelValue={next => { shown.value = next }}>
          {{
            default: (tabs: TabsScope<Shown>) => (
              <div>
                <div {...tabs.list} class="mb-8 toggle-row">
                  {SHOWN.map(format => (
                    <button key={format} {...tabs.trigger(format)} class="toggle-pill" type="button">
                      {format === 'diff' ? '--diff' : format}
                    </button>
                  ))}
                </div>
                <div class="grid">
                  {SHOWN.map(format => (
                    <div
                      key={format}
                      {...tabs.content(format)}
                      class="col-start-1 row-start-1 min-w-0 inert:invisible"
                      hidden={false}
                      inert={format !== shown.value}
                    >
                      {format === 'diff'
                        ? <DiffSample />
                        : [...records(format), format === 'json' ? <div class="pl-10" innerHTML={data.summary} /> : null]}
                    </div>
                  ))}
                </div>
              </div>
            )
          }}
        </Tabs>
      </figure>
    )
  }
})
