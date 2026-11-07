import { refAutoReset, useClipboard, useStorage }                   from '@vueuse/core'
import { computed, defineComponent, onMounted, ref, withModifiers } from 'vue'

import { CopyButton, Disclosure } from '../../../kit'
import type { DisclosureScope }   from '../../../kit'
import { ProseSandboxControls }   from './ProseSandboxControls'
import { ProseSandboxSurface }    from './ProseSandboxSurface'
import { ProseSandboxToml }       from './ProseSandboxToml'

import { useProseSandbox } from '../../../lib/composables/use-prose-sandbox'
import { data as schema }  from '../../../lib/sandbox/config-schema.data'
import { data as pool }    from '../../../lib/sandbox/pool.data'
import { counted }         from '../../../lib/shared/numerals'

export const ProseSandbox = defineComponent({
  name: 'ProseSandbox',
  setup() {
    const deckLocked   = useStorage('prose-sandbox-pinned', false)
    const deckOpen     = useStorage('prose-sandbox-deck-open', true)
    const editing      = ref(false)
    const guide        = ref<{ hue: string, value: number } | null>(null)
    const refreshArmed = refAutoReset(false, 4000)
    const sandbox      = useProseSandbox({ cases: pool.cases, schema })

    function onDragging(key: string, hue: string): void {
      guide.value = key ? { hue, value: sandbox.lengthValue(key) } : null
    }

    function onPreview(_key: string, value: number): void {
      if (guide.value) guide.value = { ...guide.value, value }
    }

    const { copied: linkCopied, copy: copyLink } = useClipboard()

    async function shareLink(): Promise<void> {
      const url = await sandbox.share()
      if (url) copyLink(url)
    }

    // An edited source or a moved config is work worth guarding, so a dirty
    // refresh arms first and discards only on the confirming second click. A
    // source matching any pool example verbatim is nobody's work, whichever
    // session it was restored from, so it refreshes without the arm.
    const dirty = computed(() =>
      sandbox.configToml.value !== '' ||
      !pool.cases.some(entry => entry.source === sandbox.source.value))

    const announcement = computed(() => {
      const findings = sandbox.diagnostics.value.length
      const lines    = sandbox.formatted.value.trimEnd().split('\n').length
      return `Formatted ${counted(lines, 'line')}, ${counted(findings, 'lint finding')}.`
    })

    function refresh(): void {
      if (refreshArmed.value || !dirty.value) {
        refreshArmed.value = false
        sandbox.refresh()
        return
      }
      refreshArmed.value = true
    }

    // The wasm glue imports client-side only, so the first random case formats
    // on mount rather than on the static-generation path.
    onMounted(sandbox.start)

    return () => (
      <div
        class="relative left-1/2 w-sandbox -translate-x-1/2"
        data-editing={editing.value || null}
      >
        <section
          class="relative z-20 mb-1-5rem panel bg-bg-soft transition-opacity in-data-editing:pointer-events-none in-data-editing:opacity-50 data-locked:sticky data-locked:top-nav max-tablet:data-locked:static"
          data-locked={deckLocked.value || undefined}
        >
          <Disclosure onUpdate:open={next => { deckOpen.value = next }} open={deckOpen.value}>
            {{
              default: (disclosure: DisclosureScope) => (
                <>
                  <button
                    {...disclosure.trigger}
                    class="group flex w-full items-center justify-center border-0 p-0-45rem text-text-2 transition-colors hover:bg-text-1/4 hover:text-text-1 focus-visible:-outline-offset-2 aria-expanded:border-b-1 aria-expanded:border-b-divider"
                    title={deckOpen.value ? 'Hide configuration' : 'Show configuration'}
                    type="button"
                  >
                    <span class="kicker group-hover:text-text-2">Configuration</span>
                  </button>
                  <button
                    aria-label={deckLocked.value ? 'Unpin the configuration' : 'Pin the configuration while scrolling'}
                    aria-pressed={deckLocked.value}
                    class="absolute top-4 right-8 z-1 border-none p-6 text-text-3 transition-colors hover:text-text-1 focus-visible:outline-offset-1 aria-pressed:text-brand-1 max-tablet:hidden"
                    onClick={withModifiers(() => { deckLocked.value = !deckLocked.value }, ['stop'])}
                    title={deckLocked.value ? 'Unpin from scroll' : 'Pin while scrolling'}
                    type="button"
                  >
                    <svg aria-hidden="true" class="size-15 glyph-stroke" fill="none" viewBox="0 0 24 24">
                      <rect height="9" rx="2" width="14" x="5" y="11" />
                      <path d={deckLocked.value ? 'M8 11V7a4 4 0 0 1 8 0v4' : 'M8 11V7a4 4 0 0 1 8 0'} />
                    </svg>
                  </button>
                  <div {...disclosure.content} class="px-1-25rem pt-0-75rem pb-1-25rem">
                    <ProseSandboxControls onDragging={onDragging} onPreview={onPreview} sandbox={sandbox} />
                  </div>
                </>
              )
            }}
          </Disclosure>
        </section>
        <div class="grid grid-cols-2fr-1fr items-stretch gap-1-25rem *:min-h-30rem max-tablet:grid-cols-1fr">
          <div class="sandbox-py copy-host relative grid min-w-0">
            <ProseSandboxSurface
              editing={editing.value}
              guide={guide.value?.value ?? null}
              guideHue={guide.value?.hue}
              onUpdate:editing={next => { editing.value = next }}
              sandbox={sandbox}
            />
            <CopyButton label="Copy the formatted Python" source={sandbox.formatted.value} style={{ display: editing.value ? 'none' : undefined }} />
            <button
              aria-label={refreshArmed.value ? 'Click again to proceed with a new example' : 'New example'}
              class="panel-corner right-40 panel-seat size-28 data-armed:border-eureka-page-35 data-armed:text-eureka-page-45 data-armed:opacity-100"
              data-armed={refreshArmed.value || null}
              onClick={refresh}
              style={{ display: editing.value ? 'none' : undefined }}
              title={refreshArmed.value ? 'Click again to proceed with a new example' : 'New example'}
              type="button"
            >
              <svg aria-hidden="true" class="glyph" fill="none" viewBox="0 0 24 24">
                <path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8" />
                <path d="M21 3v5h-5" />
                <path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16" />
                <path d="M8 16H3v5" />
              </svg>
            </button>
            <button
              aria-label={linkCopied.value ? 'Link copied' : 'Copy a link to this sandbox'}
              class="panel-corner right-72 panel-seat size-28"
              onClick={shareLink}
              style={{ display: editing.value ? 'none' : undefined }}
              title={linkCopied.value ? 'Link copied' : 'Copy a link to this sandbox'}
              type="button"
            >
              <svg aria-hidden="true" class="glyph" fill="none" viewBox="0 0 24 24">
                {linkCopied.value
                  ? <path d="M4 12.5l5 5L20 6.5" />
                  : (
                  <>
                    <path d="M10 13a5 5 0 0 0 7.5.5l3-3a5 5 0 0 0-7-7l-1.7 1.7" />
                    <path d="M14 11a5 5 0 0 0-7.5-.5l-3 3a5 5 0 0 0 7 7l1.7-1.7" />
                  </>
                )}
              </svg>
            </button>
            {refreshArmed.value ? (
              <span
                class="pointer-events-none absolute top-8 right-108 z-3 flex h-28 items-center rounded-sm border-1 border-eureka-page-35 bg-bg px-10 font-mono text-xxs tracking-0-04em whitespace-nowrap text-text-1 shadow-tooltip"
                role="status"
              > Click again to proceed with a new example </span>
            ) : null}
            <p aria-live="polite" class="sr-only">{announcement.value}</p>
          </div>
          <ProseSandboxToml sandbox={sandbox} />
        </div>
      </div>
    )
  }
})
