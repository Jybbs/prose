import { computed, defineComponent, useModel } from 'vue'
import type { PropType }                       from 'vue'

import { Listbox }           from '../../../kit'
import type { ListboxScope } from '../../../kit'

import type { SelectOption } from '../../../lib/reference/run-summary'

export const RunSummarySelect = defineComponent({
  name  : 'RunSummarySelect',
  props : {
    label      : { required: true, type: String },
    modelValue : { required: true, type: String },
    options    : { required: true, type: Array as PropType<readonly SelectOption[]> }
  },
  emits : { 'update:modelValue': (value: string) => typeof value === 'string' },
  setup(props) {
    const model = useModel(props, 'modelValue')

    const selected = computed(() =>
      props.options.find(o => o.id === model.value) ?? props.options[0]
    )

    return () => (
      <Listbox
        label={props.label}
        modelValue={model.value}
        onUpdate:modelValue={next => { model.value = next }}
        options={props.options}
      >
        {{
          default: (listbox: ListboxScope<SelectOption>) => (
            <span class="relative inline-block">
              <button
                {...listbox.trigger}
                class="run-summary-select-trigger inline-flex items-baseline gap-0-2em border-none px-2 font-mono text-link hairline-ube-38 transition-color-and-box-shadow hover:text-text-1 hover:hairline-ube focus-visible:outline-1 focus-visible:outline-ube"
                type="button"
              >
                <span>{selected.value.mono}</span>
                <span aria-hidden="true" class="text-0-7em text-ube-mid">▾</span>
              </button>
              <div {...listbox.positioner}>
                <ul
                  {...listbox.content}
                  class="run-summary-opts z-10000 m-0 w-max max-w-viewport list-none rounded-sm border-1 border-divider bg-bg p-4 shadow-tooltip"
                >
                  {props.options.map(o => (
                    <li class="mt-0" key={o.id} role="presentation">
                      <button
                        {...listbox.item(o)}
                        class="run-summary-opt group flex w-full flex-col gap-3 rounded-sm border-none px-11 py-7 text-start transition-colors not-aria-selected:hover:bg-bg-soft not-aria-selected:data-highlighted:bg-bg-soft aria-selected:bg-ube/12"
                        type="button"
                      >
                        <span class="run-summary-opt-mono font-mono text-sm text-text-1 group-aria-selected:text-link">{o.mono}</span>
                        {o.preview && (
                          <span class="inline-flex items-baseline gap-0-5em font-mono text-sm">
                            {o.preview.anchor && <span aria-hidden="true" class="flex-none">{o.preview.anchor}</span>}
                            <span
                              class="text-text-2 data-[tint=apricot]:text-apricot-code-45 data-[tint=celadon]:text-celadon-code-45 group-aria-selected:data-[tint=apricot]:text-apricot-selected-45 group-aria-selected:data-[tint=celadon]:text-celadon-selected-45"
                              data-tint={o.preview.countTint || undefined}
                            >{o.preview.text}</span>
                          </span>
                        )}
                      </button>
                    </li>
                  ))}
                </ul>
              </div>
            </span>
          )
        }}
      </Listbox>
    )
  }
})
