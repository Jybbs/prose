import { defineComponent } from 'vue'

import { Kicker } from '../../../kit'

export const LandingSection = defineComponent({
  name  : 'LandingSection',
  props : {
    centered : Boolean,
    heading  : { required: true, type: String },
    kicker   : String
  },
  setup(props, { slots }) {
    return () => (
      <div class="landing-section mt-48 max-mobile:mt-54">
        <div class={['mb-34 gap-24', props.centered ? 'mx-auto max-w-720 items-center text-center' : 'flex items-end']}>
          {props.kicker && (
            <Kicker variant="rule">
              {{ default: () => props.kicker }}
            </Kicker>
          )}
          <h2
            class="mt-12 border-b-0 font-display text-landing-h2 leading-1-1 font-500 tracking-0 text-balance text-text-1"
            innerHTML={props.heading}
          />
          {slots['heading-extra']?.()}
        </div>
        {slots.default?.()}
      </div>
    )
  }
})
