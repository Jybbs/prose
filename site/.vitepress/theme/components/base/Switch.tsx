import { defineComponent } from 'vue'

// Draws a two-state button in the `switch` role, its thumb at the track's end
// while `checked` holds, emitting `toggle` on each press.
export const Switch = defineComponent({
  name  : 'Switch',
  props : {
    checked : { required: true, type: Boolean },
    label   : { required: true, type: String }
  },
  emits : { toggle: () => true },
  setup(props, { emit }) {
    return () => (
      <button
        aria-checked={props.checked}
        aria-label={props.label}
        class="group/switch relative h-22 w-40 rounded-pill border-none bg-text-3/40 transition-colors focus-visible:outline-offset-1 aria-checked:bg-text-1"
        onClick={() => emit('toggle')}
        role="switch"
        type="button"
      >
        <span class="absolute top-3 left-3 size-16 rounded-half bg-bg transition-transform group-aria-checked/switch:translate-x-18" />
      </button>
    )
  }
})
