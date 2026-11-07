import { defineComponent, ref, Transition } from 'vue'

import { data as releases } from '../../../lib/landing/pypi-releases.data'
import { PYPI_PACKAGE }     from '../../../lib/shared/constants'

import { CopyButton, Disclosure } from '../../../kit'
import type { DisclosureScope }   from '../../../kit'
import { ReleaseLedger }          from './ReleaseLedger'
import { ReleaseStamp }           from './ReleaseStamp'
import { ReleaseToggle }          from './ReleaseToggle'

export const Cta = defineComponent({
  name: 'Cta',
  setup() {
    const current = releases[0]
    const extras  = releases.slice(1)
    const open    = ref(false)

    const installCmd = `uv tool install ${PYPI_PACKAGE}`

    return () => (
      <section class="landing-cta relative mt-76 grid grid-cols-196-minmax-1fr overflow-hidden border-1 border-divider bg-bg-soft text-left max-card-stack:grid-cols-1fr">
        <Disclosure onUpdate:open={next => { open.value = next }} open={open.value}>
          {{
            default: (disclosure: DisclosureScope) => (
              <aside class="relative min-h-200 overflow-hidden border-r-1 border-r-panel-edge bg-panel-bg max-card-stack:border-b-1 max-card-stack:border-r-0 max-card-stack:border-r-current max-card-stack:border-b-panel-edge">
                <Transition
                  enterFromClass="translate-y-8 opacity-0"
                  leaveToClass="translate-y-8 opacity-0"
                  mode="out-in"
                  name="landing-cta-swap"
                >
                  {{
                    default: () => (
                      !open.value
                          ? (
                            <div
                              class="absolute inset-0 flex flex-col items-center justify-center px-14 py-16"
                              key="stamp"
                            >
                              <ReleaseStamp release={current} />
                            </div>
                          )
                          : (
                          <div
                            key="open"
                            {...disclosure.content}
                            class="absolute inset-0 flex flex-col items-center justify-center px-14 py-16"
                          >
                            <ReleaseLedger releases={extras} />
                          </div>
                        )
                    )
                  }}
                </Transition>
                <ReleaseToggle {...disclosure.trigger} open={open.value} />
              </aside>
            )
          }}
        </Disclosure>
        <div class="relative grid grid-cols-minmax-1fr-auto items-center gap-x-18 gap-y-4 px-22 py-18">
          <p class="col-start-1 col-end-3 kicker mb-4">Read on</p>
          <p class="col-start-1 col-end-3 mb-10 max-w-48ch font-display text-1-25rem leading-1-3 font-400 text-text-1">
            {' '}
            Bring
            {' '}
            <em>Prose</em>
            {' '}
            to your own pages and make the next save
            {' '}
            <em>legible</em>
            .
            {' '}
          </p>
          <div
            aria-label="Install command"
            class="landing-cta-cmd copy-host relative col-1 inline-flex items-center gap-10 rounded-sm border-1 border-divider bg-bg py-8 pr-44 pl-14 font-mono text-code text-text-1"
          >
            <span aria-hidden="true" class="text-text-3">$</span>
            <code>{installCmd}</code>
            <CopyButton copiedDuring={2000} label="Copy install command" source={installCmd} />
          </div>
          <a
            class="group col-2 inline-flex items-center gap-6 rounded-sm bg-brand-1 px-16 py-8 font-display text-md font-500 text-bg italic transition-colors hover:bg-brand-2"
            href="/guide/quick-start"
          >
            <span>Quick start</span>
            <span aria-hidden="true" class="transition-transform group-hover:translate-x-3">→</span>
          </a>
        </div>
      </section>
    )
  }
})
