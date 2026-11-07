import { defineComponent, ref, watch } from 'vue'

import { useSettledMeasure } from '../../lib/composables/use-settled-measure'
import { middleEllipsis }    from '../../lib/shared/middle-ellipsis'

// Shortens `text` from its middle to fit its span, keeping `tail` characters
// after the ellipsis.
export const MiddleEllipsis = defineComponent({
  name  : 'MiddleEllipsis',
  props : {
    tail : { default: 3, type: Number },
    text : { required: true, type: String }
  },
  setup(props) {
    const el      = ref<HTMLSpanElement>()
    const display = ref(props.text)

    function fit(): void {
      const span = el.value
      if (!span) return
      const fits = (candidate: string): boolean => {
        span.textContent = candidate
        return span.scrollWidth <= span.clientWidth + 1
      }
      display.value = middleEllipsis(fits, props.tail, props.text)
      // The probes above mutated the span directly, so restore the chosen text
      // even when the ref value is unchanged and Vue skips the patch.
      span.textContent = display.value
    }

    useSettledMeasure(el, fit)
    watch(() => props.text, fit)

    return () => <span class="overflow-hidden whitespace-nowrap" ref={el}>{display.value}</span>
  }
})
