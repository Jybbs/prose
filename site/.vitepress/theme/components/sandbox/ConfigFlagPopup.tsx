import { useEventListener, useTimeoutFn }                       from '@vueuse/core'
import { computed, defineComponent, Fragment, ref, shallowRef } from 'vue'
import type { PropType }                                        from 'vue'

import { Popup } from '../../../kit'

import { useDismissedHover }       from '../../../lib/composables/use-dismissed-hover'
import { noticeKey, rankedFacets } from '../../../lib/sandbox/config-decorations'
import type { RuleControl }        from '../../../lib/sandbox/config-schema.data'

const GRACE_MS = 120

export interface ConfigFlagPopupHandle {
  close : () => void
  hide  : () => void
  press : (flag: HTMLElement) => void
  show  : (event: Event) => void
}

export const ConfigFlagPopup = defineComponent({
  name  : 'ConfigFlagPopup',
  props : {
    rules : { required: true, type: Array as PropType<readonly RuleControl[]> }
  },
  emits : {
    apply : (path: string, replacement: string) => typeof path === 'string' && typeof replacement === 'string'
  },
  setup(props, { emit, expose }) {
    const hover  = useDismissedHover()
    const path   = ref('')
    const target = shallowRef<HTMLElement | null>(null)

    let pressed = false

    const closing = useTimeoutFn(() => { path.value = '' }, GRACE_MS, { immediate: false })

    // A path that names a rule offers that rule's facets, closest spelling first.
    // A root key names no rule, so it offers none.
    const facets = computed(() => rankedFacets(path.value, props.rules))

    // The path reads as a trail from the prose table down to the segment that
    // failed, so the hierarchy shows what the key was written under.
    const trail = computed(() => path.value.split('.').slice(0, -1))
    const wrote = computed(() => path.value.split('.').at(-1) ?? '')

    function flagFrom(event: Event): HTMLElement | null {
      const node = event.target
      return node instanceof Element ? node.closest<HTMLElement>('.lint-flag') : null
    }

    // The anchors sit inside HTML set through `innerHTML`, so event delegation finds the
    // hovered key and the popup takes it as its anchor.
    function show(event: Event): void {
      const flag = flagFrom(event)
      if (hover.enter(flag) || !flag) return
      pressed = false
      point(flag)
    }

    // Shows the card of a tapped key and holds it open past the hover's leave.
    function press(flag: HTMLElement): void {
      pressed = true
      point(flag)
    }

    // Anchors the card to a key. A repeat on the flag already shown returns before
    // the read, since crossing between a flag's own token spans re-fires the
    // delegated hover.
    function point(flag: HTMLElement): void {
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
      hover.leave()
      if (!pressed && !closing.isPending.value) closing.start()
    }

    function hold(): void {
      closing.stop()
    }

    function close(): void {
      hold()
      path.value = ''
    }

    // Closes the card on Escape or an interaction outside it, leaving focus
    // where it is.
    function dismiss(): void {
      close()
      hover.dismiss()
    }

    function apply(replacement: string): void {
      emit('apply', path.value, replacement)
      close()
    }

    // Keeps focus in the editor through a press on the card of a key tapped there.
    // A blur hides the editor, and with it the key the card is anchored to.
    function keepFocus(event: Event): void {
      if (pressed) event.preventDefault()
    }

    useEventListener('pointermove', event => { hover.move(flagFrom(event)) })

    expose({ close, hide, press, show })

    return () => (
      <Popup
        anchor={target.value}
        class="glossary-popup"
        label={path.value}
        onDismiss={dismiss}
        open={path.value !== ''}
        overflowPadding={16}
        passive
        theme="glossary"
      >
        {{
          default: () => (
            <div
              class="flex max-w-42ch flex-col gap-12 px-2 py-4"
              onMousedown={keepFocus}
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
                {facets.value.length === 0 ? (
                  <>
                    <span aria-hidden="true" class="text-text-3">›</span>
                    <span class="text-danger-1 line-through">{wrote.value}</span>
                  </>
                ) : null}
              </p>
              {facets.value.length > 0 ? (
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
              ) : null}
            </div>
          )
        }}
      </Popup>
    )
  }
})
