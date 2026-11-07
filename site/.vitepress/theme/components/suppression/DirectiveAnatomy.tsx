import { defineComponent } from 'vue'

import { ScrollRegion, Tabs } from '../../../kit'
import type { TabsScope }     from '../../../kit'

import { data as directives }     from '../../../lib/suppression/directives.data'
import { useTabSelect }           from '../../../lib/composables/use-tab-select'
import type { PartRole }          from '../../../lib/suppression/directive-parts'
import { SCOPE_META, scopeBands } from '../../../lib/suppression/scopes'
import { InlineProse }            from '../base/InlineProse'

const ROLE_LABEL: Record<PartRole, string> = {
  action    : 'action',
  comment   : 'comment',
  namespace : 'namespace',
  payload   : 'rule list'
}

export const DirectiveAnatomy = defineComponent({
  name: 'DirectiveAnatomy',
  setup() {
    const bands = scopeBands(directives)

    const { active: focused, selected: focusId } = useTabSelect(directives, d => d.id)
    focusId.value = 'prose-ignore-rules'

    function classifyLine(line: string): 'directive' | 'comment' | 'code' {
      const trimmed = line.trim()
      if (trimmed.includes(' fmt:') || trimmed.includes(' yapf:') || trimmed.includes(' prose:')) {
        return 'directive'
      }
      if (trimmed.startsWith('#')) return 'comment'
      return 'code'
    }

    return () => (
      <Tabs
        label="Directive index"
        modelValue={focusId.value}
        onUpdate:modelValue={next => { focusId.value = next }}
      >
        {{
          default: (tabs: TabsScope<string>) => (
            <div class="directive-anatomy overflow-hidden panel bg-panel-wash">
              <div {...tabs.list} class="flex flex-col gap-20 px-20 pt-18 pb-22">
                {bands.map(band => (
                  <section
                    class="directive-anatomy-band flex flex-col gap-8"
                    data-scope={band.scope}
                    key={band.scope}
                  >
                    <header class="flex items-center gap-9 px-2">
                      <span
                        aria-hidden="true"
                        class="size-22 shrink-0 text-0-92rem leading-none inline-flex items-center justify-center bg-scope-mark font-display font-500 text-woodsmoke italic rounded-half"
                      >{SCOPE_META[band.scope].pip}</span>
                      <span class="anatomy-band-name">{SCOPE_META[band.scope].label}</span>
                      <span aria-hidden="true" class="anatomy-band-rule" />
                    </header>
                    <div class="flex flex-wrap gap-6">
                      {band.items.map(d => (
                        <button
                          key={d.id}
                          {...tabs.trigger(d.id)}
                          class="directive-anatomy-thumb group inline-flex items-center rounded-3 border-1 px-9 pt-2 pb-3 transition-colors focus-visible:focus-ring focus-visible:outline-offset-1"
                          data-active={focusId.value === d.id || undefined}
                          data-scope={d.scope}
                          type="button"
                        >
                          <span class="border-none font-mono text-xxs leading-1-5 text-text-2 group-data-active:text-text-1 group-hover:text-text-1 group-focus-visible:text-text-1">{d.form}</span>
                        </button>
                      ))}
                    </div>
                  </section>
                ))}
              </div>
              <div
                {...tabs.content(focusId.value)}
                class="directive-anatomy-plate relative grid grid-cols-1fr gap-14 px-32 pt-18 pb-22"
                data-scope={focused.value.scope}
              >
                <div class="flex flex-wrap items-start gap-0">
                  {focused.value.parts.map((part, i) => (
                    <span
                      class="group grid grid-rows-auto-28-auto items-start justify-items-center px-6 text-center"
                      data-role={part.role}
                      key={i}
                    >
                      <span class="directive-anatomy-part border-none font-mono text-anatomy-part leading-1-15 whitespace-pre text-part-or-text-1 group-data-[role=action]:font-600 group-data-[role=action]:text-scope-ink">{part.text}</span>
                      <span aria-hidden="true" class="my-2 h-24 w-1 bg-text-2" />
                      <span class="font-display text-0-92rem italic text-text-2">{ROLE_LABEL[part.role]}</span>
                    </span>
                  ))}
                </div>
                <div class="flex flex-col gap-4">
                  <p class="directive-anatomy-effect max-w-72ch text-0-98rem text-text-2 italic in-code:not-italic">
                    <InlineProse nodes={focused.value.effectNodes} />
                  </p>
                  <ScrollRegion as="pre" class="overflow-x-auto rounded-sm border-1 border-divider bg-bg px-14 py-12 text-code leading-1-55 text-text-2">
                    <code>
                      {focused.value.example.split('\n').map((line, j) => (
                        <span
                          class="block data-[line-kind=code]:text-text-1 data-[line-kind=comment]:text-text-3 data-[line-kind=directive]:font-500 data-[line-kind=directive]:text-text-1"
                          data-line-kind={classifyLine(line)}
                          key={j}
                        >{line}{"\n"}</span>
                      ))}
                    </code>
                  </ScrollRegion>
                </div>
              </div>
            </div>
          )
        }}
      </Tabs>
    )
  }
})
