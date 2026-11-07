import { defineComponent } from 'vue'

import { Tabs, ToolIcon } from '../../../kit'
import type { TabsScope } from '../../../kit'

import { data as editors }   from '../../../lib/integrations/editor-configs.data'
import { useTabOrientation } from '../../../lib/composables/use-tab-orientation'
import { useTabSelect }      from '../../../lib/composables/use-tab-select'
import { formatFolio }       from '../../../lib/shared/numerals'

export const EditorRunOnSave = defineComponent({
  name: 'EditorRunOnSave',
  setup() {
    const { selected: activeSlug } = useTabSelect(editors, e => e.slug)
    const orientation              = useTabOrientation()

    return () => (
      <Tabs
        label="Editors"
        modelValue={activeSlug.value}
        onUpdate:modelValue={next => { activeSlug.value = next }}
        orientation={orientation.value}
      >
        {{
          default: (tabs: TabsScope<string>) => (
            <div class="my-1-5rem grid grid-cols-232-1fr overflow-hidden panel bg-bg max-card-stack:grid-cols-1fr">
              <div class="border-r-1 border-r-divider bg-bg-soft pt-14 pr-14 pb-18 pl-18 max-card-stack:border-b-1 max-card-stack:border-r-0 max-card-stack:border-r-current max-card-stack:border-b-divider">
                <span class="kicker mb-14 block pb-13 border-b-double-divider">Editors · {formatFolio(editors.length)}</span>
                <ul
                  {...tabs.list}
                  class="m-0 flex list-none flex-col gap-2 p-0 max-card-stack:flex-row max-card-stack:flex-wrap max-card-stack:gap-4"
                >
                  {editors.map((editor, i) => (
                    <li class="mt-0" key={editor.slug} role="presentation">
                      <button
                        {...tabs.trigger(editor.slug)}
                        class="editor-card-row group flex w-full items-center gap-10 border-0 border-l-2 border-l-transparent p-8 text-left text-text-2 transition-colors aria-selected:border-l-link aria-selected:text-text-1 max-card-stack:flex-3/10"
                        type="button"
                      >
                        <span class="folio">№ {formatFolio(i + 1)}</span>
                        <span class="tool-ring size-22 transition-colors group-aria-selected:text-link">
                          <ToolIcon class="size-12" slug={editor.slug} />
                        </span>
                        <span class="min-w-0 overflow-hidden font-display text-md font-500 whitespace-nowrap italic group-aria-selected:font-600 max-card-stack:group-aria-selected:font-500">{editor.name}</span>
                        <span aria-hidden="true" class="leader-fill max-card-stack:hidden" />
                      </button>
                    </li>
                  ))}
                </ul>
              </div>
              <div aria-live="polite" class="flex min-w-0 flex-col">
                {editors.map(editor => (
                  <section
                    key={editor.slug}
                    {...tabs.content(editor.slug)}
                    class="editor-card-face flex flex-1 flex-col bg-code-block"
                  >
                    <header class="flex items-baseline gap-10 border-b-1 border-b-divider px-18 py-14">
                      <span class="kicker">{editor.caption}</span>
                      <span class="editor-card-face-target ml-auto font-mono text-xs text-text-2">{editor.target}</span>
                    </header>
                    <div class="flex flex-1 flex-col flush-code-block *:flex-1" innerHTML={editor.codeHtml} />
                  </section>
                ))}
              </div>
            </div>
          )
        }}
      </Tabs>
    )
  }
})
