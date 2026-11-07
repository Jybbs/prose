import { defineComponent } from 'vue'

export const ReleaseToggle = defineComponent({
  name  : 'ReleaseToggle',
  props : {
    open : { required: true, type: Boolean }
  },
  emits : ['toggle'],
  setup(props, { emit }) {
    return () => (
      <button
        aria-expanded={props.open}
        aria-label={props.open ? 'Back to current release' : 'Show previous releases'}
        class="absolute top-8 right-8 z-5 medallion size-26 border-stamp-accent-30-panel-edge bg-stamp-accent-8-panel-bg font-mono text-md leading-none text-stamp-ink transition-background-border-color-and-scale hover:scale-106 hover:border-stamp-accent-55-panel-edge hover:bg-stamp-accent-22-bg"
        onClick={() => emit('toggle')}
        type="button"
      >{props.open ? '←' : '+'}</button>
    )
  }
})
