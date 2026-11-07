import { defineComponent } from 'vue'


export const FixtureNoChange = defineComponent({
  name: 'FixtureNoChange',
  setup() {
    return () => (
      <div class="fixture-nochange relative inline-flex">
        <div aria-hidden="true" class="fixture-nochange-ghost">
          <span class="fixture-toggle-pill toggle-pill" data-side="before">Before</span>
          <span class="fixture-toggle-pill toggle-pill" data-side="after">After</span>
        </div>
        <span class="fixture-nochange-label">No Change</span>
      </div>
    )
  }
})
