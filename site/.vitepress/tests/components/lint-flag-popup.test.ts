// @vitest-environment happy-dom
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'

import { LintFlagPopup, type LintFlagPopupHandle } from '../../theme/components/rules/LintFlagPopup'

vi.mock('../../lib/shared/highlight', () => import('../highlight-stub'))

vi.mock('../../lib/rules/rules.data', async () =>
  (await import('../rules-data-stub')).rulesDataStub([
    { family: 'lint', slug: 'line-overflow' },
    { family: 'lint', slug: 'miscased-constants' },
    { family: 'lint', slug: 'signature-annotations' },
    { family: 'lint', slug: 'step-narration' }
  ]))

type Popup  = ReturnType<typeof mountPopup>

const stubs = { teleport: true, RuleCard: { template: '<div class="rule-card"><slot name="header" /></div>' } }

const pressOn = (target: Element | null) => ({ preventDefault: vi.fn<() => void>(), target }) as unknown as Event

// The `.lint-flag` anchors live in `v-html` output rather than in the
// component's own tree, so each case builds one and hands it over the way
// the delegated press does.
function press(popup: Popup, data: Record<string, string>, text: string): void {
  const flag       = document.createElement('span')
  flag.className   = 'lint-flag'
  flag.textContent = text
  Object.assign(flag.dataset, data)
  document.body.append(flag)
  popup.vm.toggle(pressOn(flag))
}

function mountPopup() {
  return mount(LintFlagPopup, { global: { stubs } }) as unknown as VueWrapper<LintFlagPopupHandle>
}

describe('LintFlagPopup', () => {
  afterEach(() => {
    document.querySelectorAll('.lint-flag').forEach(flag => flag.remove())
  })

  it('holds the popup clear of the viewport edge', () => {
    expect(mountPopup().getComponent({ name: 'Popup' }).props('overflowPadding')).toBe(16)
  })

  it('renders the chip pair for a single-token replacement', async () => {
    const popup  = mountPopup()
    press(popup, { message: 'm', rule: 'miscased-constants', suggested: 'MAX_RETRIES' }, 'max_retries')
    await flushPromises()
    expect(popup.get('.lint-chip-struck').text()).toBe('max_retries')
    expect(popup.get('.lint-chip-suggest').text()).toBe('MAX_RETRIES')
  })

  it('renders the anchor and the added text for an insert', async () => {
    const popup  = mountPopup()
    press(popup, { message: 'm', rule: 'signature-annotations', suggested: ': int' }, 'count')
    await flushPromises()
    expect(popup.get('.lint-chip-insert').text()).toBe('count: int')
    expect(popup.get('.lint-inserted').text()).toBe(': int')
  })

  it('stacks the highlighted before and after for a multi-line replacement', async () => {
    const popup  = mountPopup()
    press(popup, {
      message   : 'm',
      replaced  : '"ab"',
      rule      : 'line-overflow',
      suggested : '(\n    "a"\n    "b"\n)'
    }, 'BANNER = "ab"')
    await flushPromises()
    const before = popup.get('.lint-block-side[data-side="before"]')
    const after  = popup.get('.lint-block-side[data-side="after"]')
    expect(before.get('.lint-block-label').text()).toBe('Before')
    expect(before.get('.lint-block-code').html()).toContain('pre class="shiki"')
    expect(after.get('.lint-block-code').text()).toContain('"a"')
    expect(after.get('.lint-block-code').attributes('tabindex')).toBe('0')
  })

  it('holds the message in the header until the panes are highlighted', () => {
    const popup  = mountPopup()
    press(popup, {
      message   : 'Line is 54 columns',
      replaced  : '"ab"',
      rule      : 'line-overflow',
      suggested : '(\n    "a"\n)'
    }, 'BANNER = "ab"')
    expect(popup.find('.lint-block').exists()).toBe(false)
  })

  it('drops a superseded pane render rather than landing it behind the newer flag', async () => {
    const popup  = mountPopup()
    press(popup, {
      message   : 'm',
      replaced  : '"ab"',
      rule      : 'line-overflow',
      suggested : '(\n    "a"\n)'
    }, 'BANNER = "ab"')
    press(popup, { message: 'm', rule: 'step-narration' }, '# Step 1: parse')
    await flushPromises()
    expect(popup.find('.lint-block').exists()).toBe(false)
    expect(popup.get('.lint-chip-struck').text()).toBe('# Step 1: parse')
  })

  it('reopens the card when the same flag is pressed after a hide', async () => {
    const popup  = mountPopup()
    press(popup, { message: 'm', rule: 'signature-annotations', suggested: ': int' }, 'count')
    await flushPromises()
    popup.vm.hide()
    await flushPromises()

    popup.vm.toggle(pressOn(document.querySelector('.lint-flag')))
    await flushPromises()
    expect(popup.get('.lint-chip-insert').text()).toBe('count: int')
  })

  it('leaves the card closed for a flag naming an unregistered rule', async () => {
    const popup  = mountPopup()
    press(popup, { message: 'm', rule: 'ghost-rule' }, 'x')
    await flushPromises()
    expect(popup.find('.rule-card').exists()).toBe(false)
  })

  it('closes the card on a second press of the same flag', async () => {
    const popup  = mountPopup()
    press(popup, { message: 'm', rule: 'signature-annotations', suggested: ': int' }, 'count')
    await flushPromises()
    const event = pressOn(document.querySelector('.lint-flag'))
    expect(popup.vm.toggle(event)).toBe(true)
    await flushPromises()
    expect(popup.find('.rule-card').exists()).toBe(false)
  })

  it('reports a press outside any flag as not its own', () => {
    const popup = mountPopup()
    const event = pressOn(document.body)
    expect(popup.vm.toggle(event)).toBe(false)
  })

  it('clears the card when the popup reports a dismissal', async () => {
    const popup  = mountPopup()
    press(popup, { message: 'm', rule: 'signature-annotations', suggested: ': int' }, 'count')
    await flushPromises()
    popup.getComponent({ name: 'Popup' }).vm.$emit('dismiss')
    await flushPromises()
    expect(popup.find('.rule-card').exists()).toBe(false)
  })

  it('clears the card on hide', async () => {
    const popup  = mountPopup()
    press(popup, { message: 'm', rule: 'signature-annotations', suggested: ': int' }, 'count')
    await flushPromises()
    popup.vm.hide()
    await flushPromises()
    expect(popup.find('.rule-card').exists()).toBe(false)
  })

  it('holds a press on a flag back from the page', () => {
    const popup = mountPopup()
    press(popup, { message: 'm', rule: 'signature-annotations', suggested: ': int' }, 'count')
    const event = pressOn(document.querySelector('.lint-flag'))
    popup.vm.toggle(event)
    expect(event.preventDefault).toHaveBeenCalledOnce()
  })
})
