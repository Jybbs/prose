// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils'

import LintFlagPopper from '../../theme/components/rules/LintFlagPopper.vue'

vi.mock('../../lib/shared/highlight', () => import('../highlight-stub'))

vi.mock('floating-vue', () => ({
  PopperWrapper: {
    name     : 'PopperWrapper',
    props    : { autoBoundaryMaxSize: Boolean, overflowPadding: [Number, String], placement: String },
    template : '<div class="popper"><slot name="popper" /></div>'
  }
}))

vi.mock('../../lib/rules/rules.data', () => ({
  data: {
    bySlug: {
      'line-overflow'         : { family: 'lint', slug: 'line-overflow' },
      'miscased-constants'    : { family: 'lint', slug: 'miscased-constants' },
      'signature-annotations' : { family: 'lint', slug: 'signature-annotations' },
      'step-narration'        : { family: 'lint', slug: 'step-narration' }
    }
  }
}))

type Popper = ReturnType<typeof mountPopper>

const stubs = { RuleCard: { template: '<div class="rule-card"><slot name="header" /></div>' } }

// The `.lint-flag` anchors live in `v-html` output rather than in the
// component's own tree, so each case builds one and hands it over the way
// the delegated hover does.
function hover(popper: Popper, data: Record<string, string>, text: string): void {
  const flag       = document.createElement('span')
  flag.className   = 'lint-flag'
  flag.textContent = text
  Object.assign(flag.dataset, data)
  document.body.append(flag)
  popper.vm.show({ target: flag } as unknown as Event)
}

function mountPopper() {
  return mount(LintFlagPopper, { global: { stubs } })
}

describe('LintFlagPopper', () => {
  afterEach(() => {
    document.querySelectorAll('.lint-flag').forEach(flag => flag.remove())
  })

  it('clamps the popper against the viewport boundary', () => {
    const wrapper = mountPopper().getComponent({ name: 'PopperWrapper' })
    expect(wrapper.props('autoBoundaryMaxSize')).toBe(true)
    expect(wrapper.props('overflowPadding')).toBe(16)
  })

  it('renders the chip pair for a single-token replacement', async () => {
    const popper = mountPopper()
    hover(popper, { message: 'm', rule: 'miscased-constants', suggested: 'MAX_RETRIES' }, 'max_retries')
    await flushPromises()
    expect(popper.get('.lint-chip-struck').text()).toBe('max_retries')
    expect(popper.get('.lint-chip-suggest').text()).toBe('MAX_RETRIES')
  })

  it('renders the anchor and the added text for an insert', async () => {
    const popper = mountPopper()
    hover(popper, { message: 'm', rule: 'signature-annotations', suggested: ': int' }, 'count')
    await flushPromises()
    expect(popper.get('.lint-chip-insert').text()).toBe('count: int')
    expect(popper.get('.lint-inserted').text()).toBe(': int')
  })

  it('stacks the highlighted before and after for a multi-line replacement', async () => {
    const popper = mountPopper()
    hover(popper, {
      message   : 'm',
      replaced  : '"ab"',
      rule      : 'line-overflow',
      suggested : '(\n    "a"\n    "b"\n)'
    }, 'BANNER = "ab"')
    await flushPromises()
    const before = popper.get('.lint-block-side[data-side="before"]')
    const after  = popper.get('.lint-block-side[data-side="after"]')
    expect(before.get('.lint-block-label').text()).toBe('Before')
    expect(before.get('.lint-block-code').html()).toContain('pre class="shiki"')
    expect(after.get('.lint-block-code').text()).toContain('"a"')
  })

  it('holds the message in the header until the panes are highlighted', () => {
    const popper = mountPopper()
    hover(popper, {
      message   : 'Line is 54 columns',
      replaced  : '"ab"',
      rule      : 'line-overflow',
      suggested : '(\n    "a"\n)'
    }, 'BANNER = "ab"')
    expect(popper.find('.lint-block').exists()).toBe(false)
  })

  it('drops a superseded pane render rather than landing it behind the newer hover', async () => {
    const popper = mountPopper()
    hover(popper, {
      message   : 'm',
      replaced  : '"ab"',
      rule      : 'line-overflow',
      suggested : '(\n    "a"\n)'
    }, 'BANNER = "ab"')
    hover(popper, { message: 'm', rule: 'step-narration' }, '# Step 1: parse')
    await flushPromises()
    expect(popper.find('.lint-block').exists()).toBe(false)
    expect(popper.get('.lint-chip-struck').text()).toBe('# Step 1: parse')
  })

  it('reopens the card when the same flag is hovered after a hide', async () => {
    const popper = mountPopper()
    hover(popper, { message: 'm', rule: 'signature-annotations', suggested: ': int' }, 'count')
    await flushPromises()
    popper.vm.hide()
    await flushPromises()

    popper.vm.show({ target: document.querySelector('.lint-flag') } as unknown as Event)
    await flushPromises()
    expect(popper.get('.lint-chip-insert').text()).toBe('count: int')
  })

  it('leaves the card closed for a flag naming an unregistered rule', async () => {
    const popper = mountPopper()
    hover(popper, { message: 'm', rule: 'ghost-rule' }, 'x')
    await flushPromises()
    expect(popper.find('.rule-card').exists()).toBe(false)
  })

  it('clears the card on hide', async () => {
    const popper = mountPopper()
    hover(popper, { message: 'm', rule: 'signature-annotations', suggested: ': int' }, 'count')
    await flushPromises()
    popper.vm.hide()
    await flushPromises()
    expect(popper.find('.rule-card').exists()).toBe(false)
  })
})
