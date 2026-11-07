import { defineComponent } from 'vue'

export const FixtureConfig = defineComponent({
  name  : 'FixtureConfig',
  props : {
    html : { required: true, type: String }
  },
  setup(props) {
    return () => (
      <figure class="fixture-config mb-12">
        <figcaption class="code-caption">prose.toml</figcaption>
        <div class="overflow-hidden panel bg-code-block *:m-0" innerHTML={props.html} />
      </figure>
    )
  }
})
