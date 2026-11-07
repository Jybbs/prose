import { computed, defineComponent, Fragment, ref, shallowRef } from 'vue'
import type { PropType }                                        from 'vue'

import { Popup } from '../../../kit'

import { noticeKey, rankedFacets } from '../../../lib/sandbox/config-decorations'
import type { RuleControl }        from '../../../lib/sandbox/config-schema.data'

const GRACE_MS = 120

export interface ConfigFlagPopupHandle {
  hide : () => void
  show : (event: Event) => void
}

export const ConfigFlagPopup = defineComponent({
  name  : 'ConfigFlagPopup',
  props : {
    rules : { required: true, type: Array as PropType<readonly RuleControl[]> }
  },
  emits : { apply: (path: string, replacement: string) => true },
  setup(props, { emit, expose }) {
    const path   = ref('')
    const target = shallowRef<HTMLElement | null>(null)

    let closing: number | null = null

    // A path that names a rule offers that rule's facets, closest spelling first.
    // A root key names no rule, so it offers none.
    const facets = computed(() => rankedFacets(path.value, props.rules))

    // The path reads as a trail from the prose table down to the segment that
    // failed, so the hierarchy shows what the key was written under.
    const trail = computed(() => path.value.split('.').slice(0, -1))
    const wrote = computed(() => path.value.split('.').at(-1) ?? '')

    // The anchors sit inside `v-html` static HTML, so event delegation finds the
    // hovered key and the popup takes it as its anchor. A repeat on the flag
    // already shown returns before the read, since crossing between a flag's own
    // token spans re-fires the delegated hover.
    function show(event: Event): void {
      const flag = (event.target as HTMLElement).closest<HTMLElement>('.lint-flag')
      if (!flag) return
      hold()
      if (path.value && flag === target.value) return
      const named = noticeKey(flag.dataset.message ?? '')
      if (named === null) return
      target.value = flag
      path.value   = named
    }

    // The pointer crosses dead space on its way from the key to a replacement
    // button, so a leave schedules the close and entering the card cancels it.
    function hide(): void {
      closing ??= window.setTimeout(() => { path.value = ''; closing = null }, GRACE_MS)
    }

    function hold(): void {
      if (closing === null) return
      window.clearTimeout(closing)
      closing = null
    }

    function close(): void {
      hold()
      path.value = ''
    }

    function apply(replacement: string): void {
      emit('apply', path.value, replacement)
      close()
    }

    expose({ hide, show })

    return () => (
      <Popup
        anchor={target.value}
        label={path.value}
        open={path.value !== ''}
        overflowPadding={16}
        passive
        theme="glossary"
      >
        <div
          class="flex max-w-42ch flex-col gap-12 px-2 py-4"
          onMouseenter={hold}
          onMouseleave={close}
        >
          <p class="flex flex-wrap items-center gap-5 font-mono text-xs text-text-2">
            <span class="text-text-3">[tool.prose]</span>
            {trail.value.map(segment => (
              <Fragment key={segment}>
                <span aria-hidden="true" class="text-text-3">›</span>
                <span>{segment}</span>
              </Fragment>
            ))}
            {!facets.value.length && (
              <>
                <span aria-hidden="true" class="text-text-3">›</span>
                <span class="text-danger-1 line-through">{wrote.value}</span>
              </>
            )}
          </p>
          {facets.value.length > 0 && (
            <div class="flex flex-col gap-7">
              {facets.value.map(facet => (
                <button
                  class="group grid grid-cols-14-1fr-auto items-center gap-9 rounded-sm border-1 border-divider bg-bg px-11 py-7 text-left font-mono text-xs text-text-1 transition-colors hover:border-warning-1/55 hover:bg-warning-1/10 focus-visible:border-warning-1/55 focus-visible:bg-warning-1/10"
                  key={facet.key}
                  onClick={() => apply(facet.key)}
                  type="button"
                >
                  <span aria-hidden="true" class="text-center text-meta text-warning-1">✓</span>
                  <span>{facet.key}</span>
                  <span class="text-right text-meta text-text-3 group-hover:text-warning-1">{facet.kind} · {facet.default}</span>
                </button>
              ))}
              <p class="mt-2 grid grid-cols-14-1fr items-center gap-9 border-t-1 border-t-divider px-12 py-5 font-mono text-xs">
                <span aria-hidden="true" class="text-center text-meta text-danger-1">✕</span>
                <span class="text-danger-1 line-through">{wrote.value}</span>
              </p>
            </div>
          )}
        </div>
      </Popup>
    )
  }
})
