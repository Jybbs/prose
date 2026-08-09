// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils'
import { promiseTimeout }       from '@vueuse/core'
import { ref }                  from 'vue'

import ProseSandboxSurface   from '../../theme/components/sandbox/ProseSandboxSurface.vue'
import type { ProseSandbox } from '../../lib/composables/use-prose-sandbox'
import { nextPaint }          from '../../lib/shared/paint'
import { domTest, isHidden }  from '../dom'

const drawSettled = (): Promise<void> => promiseTimeout(550)

vi.mock('../../lib/shared/highlight', () => import('../highlight-stub'))

vi.mock('../../lib/markdown/magic-move', () => ({
  precompileMagicMove: () => Promise.resolve([{ tokens: [] }, { tokens: [] }])
}))

vi.mock('@shikijs/magic-move/vue', () => ({
  ShikiMagicMovePrecompiled: { name: 'ShikiMagicMovePrecompiled', template: '<pre class="mm" />' }
}))

const fakeSandbox = (formatted: string, source = formatted): ProseSandbox => ({
  diagnostics : ref([]),
  error       : ref(''),
  formatted   : ref(formatted),
  source      : ref(source)
} as unknown as ProseSandbox)

const FINDING = {
  code         : 'r1',
  end_location : { column: 2, row: 1 },
  location     : { column: 1, row: 1 },
  message      : 'm'
}

const LintFlagPopperStub = {
  methods  : { hide() {}, show() {} },
  template : '<div class="popper-stub" />'
}

const mountSurface = async (sandbox: ProseSandbox) => {
  const wrapper = mount(ProseSandboxSurface, {
    props  : { sandbox },
    global : { stubs: { LintFlagPopper: LintFlagPopperStub } }
  })
  await flushPromises()
  return wrapper
}

describe('ProseSandboxSurface', () => {
  domTest('opens the editor on the source rather than the formatted output', async ({ reducedMotion }) => {
    reducedMotion(false)
    const sandbox = fakeSandbox('x = 1', 'x=1')
    const wrapper = await mountSurface(sandbox)

    const display = wrapper.get('.sandbox-surface-display')
    expect(display.html()).toContain('x = 1')

    // Seeding from the formatted output would feed an already-formatted source
    // back in, leaving every rewriting rule with nothing left to do.
    await display.trigger('click')
    await flushPromises()
    expect(wrapper.get('textarea').element.value).toBe('x=1')
  })

  domTest('leaves the source alone until the reader applies the edit', async ({ reducedMotion }) => {
    reducedMotion(false)
    const sandbox = fakeSandbox('x = 1', 'x=1')
    const wrapper = await mountSurface(sandbox)

    await wrapper.get('.sandbox-surface-display').trigger('click')
    await flushPromises()
    await wrapper.get('textarea').setValue('y=2')
    await wrapper.get('textarea').trigger('blur')
    expect(sandbox.source.value).toBe('x=1')

    await wrapper.get('.sandbox-surface-apply').trigger('click')
    expect(sandbox.source.value).toBe('y=2')
    expect(isHidden(wrapper.get('.code-editor'))).toBe(true)
  })

  domTest('discards the edit and keeps the source', async ({ reducedMotion }) => {
    reducedMotion(false)
    const sandbox = fakeSandbox('x = 1', 'x=1')
    const wrapper = await mountSurface(sandbox)

    await wrapper.get('.sandbox-surface-display').trigger('click')
    await flushPromises()
    await wrapper.get('textarea').setValue('y=2')
    await wrapper.get('.sandbox-surface-discard').trigger('click')

    expect(sandbox.source.value).toBe('x=1')
    expect(isHidden(wrapper.get('.code-editor'))).toBe(true)
  })

  domTest('drives the apply-pane from the keyboard', async ({ reducedMotion }) => {
    reducedMotion(false)
    const sandbox = fakeSandbox('x = 1', 'x=1')
    const wrapper = await mountSurface(sandbox)

    // Enter on the display opens the editor on the source.
    await wrapper.get('.sandbox-surface-display').trigger('keydown.enter')
    await flushPromises()
    expect(isHidden(wrapper.get('.code-editor'))).toBe(false)
    expect(wrapper.get('textarea').element.value).toBe('x=1')

    // Esc discards without touching the source.
    await wrapper.get('textarea').setValue('discard = 1')
    await wrapper.get('textarea').trigger('keydown.esc')
    expect(sandbox.source.value).toBe('x=1')
    expect(isHidden(wrapper.get('.code-editor'))).toBe(true)

    // Ctrl+Enter applies the edit.
    await wrapper.get('.sandbox-surface-display').trigger('click')
    await wrapper.get('textarea').setValue('y=2')
    await wrapper.get('textarea').trigger('keydown.enter', { ctrlKey: true })
    expect(sandbox.source.value).toBe('y=2')
    expect(isHidden(wrapper.get('.code-editor'))).toBe(true)
  })

  domTest('keeps reacting after a config-driven reformat', async ({ reducedMotion }) => {
    reducedMotion(false)
    const sandbox = fakeSandbox('x = 1')
    const wrapper = await mountSurface(sandbox)

    sandbox.formatted.value = 'x      = 1'
    await flushPromises()
    await nextPaint()
    await flushPromises()

    sandbox.formatted.value = 'x = 2'
    await flushPromises()
    await nextPaint()
    await flushPromises()
    expect(wrapper.get('.sandbox-surface-display').html()).toContain('x = 2')
  })

  domTest('drops a disabled rule\'s squiggle while keeping the code', async ({ reducedMotion }) => {
    reducedMotion(false)
    const sandbox = fakeSandbox('x = 1')
    sandbox.diagnostics.value = [FINDING]
    const wrapper = await mountSurface(sandbox)
    expect(wrapper.get('.sandbox-surface-display').html()).toContain('data-rule="r1"')

    sandbox.diagnostics.value = []
    await flushPromises()

    // The dropped rule's underline retracts in place before the html swaps.
    expect(wrapper.get('.lint-flag[data-rule="r1"]').classes()).toContain('lint-undrawn')

    await drawSettled()
    await flushPromises()
    const display = wrapper.get('.sandbox-surface-display')
    expect(display.html()).not.toContain('data-rule="r1"')
    expect(display.html()).toContain('x = 1')
  })

  domTest('draws a newly enabled rule\'s squiggle onto unchanged code', async ({ reducedMotion }) => {
    reducedMotion(false)
    const sandbox = fakeSandbox('x = 1')
    const wrapper = await mountSurface(sandbox)
    expect(wrapper.get('.sandbox-surface-display').html()).not.toContain('data-rule="r1"')

    sandbox.diagnostics.value = [FINDING]
    await flushPromises()
    await drawSettled()
    await flushPromises()

    // The freshly enabled underline draws back in rather than staying staged.
    expect(wrapper.get('.lint-flag[data-rule="r1"]').classes()).not.toContain('lint-undrawn')
  })

  domTest('washes a whole-row finding rather than underlining it', async ({ reducedMotion }) => {
    reducedMotion(true)
    const sandbox = fakeSandbox('x = 1')
    sandbox.diagnostics.value = [{ ...FINDING, end_location: { column: 6, row: 1 } }]
    const wrapper = await mountSurface(sandbox)

    expect(wrapper.get('.lint-flag[data-rule="r1"]').classes()).toContain('lint-flag-line')
  })
})
