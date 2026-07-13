// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils'
import { ref }                  from 'vue'

import ProseSandboxSurface    from '../../theme/components/sandbox/ProseSandboxSurface.vue'
import type { ProseSandbox }  from '../../lib/composables/use-prose-sandbox'
import { domTest, nextFrame } from '../dom'

interface Decoration { properties: Record<string, string> }

const drawSettled = (): Promise<void> => new Promise(resolve => { setTimeout(resolve, 550) })

vi.mock('../../lib/sandbox/highlight', () => ({
  highlight: (code: string, _lang: string, decorations: Decoration[] = []) => {
    const flags = decorations
      .map(item => `<span class="lint-flag" data-rule="${item.properties['data-rule']}">x</span>`)
      .join('')
    return Promise.resolve(`<pre class="shiki"><code>${code}${flags}</code></pre>`)
  }
}))

vi.mock('../../lib/markdown/magic-move', () => ({
  precompileMagicMove: () => Promise.resolve([{ tokens: [] }, { tokens: [] }])
}))

vi.mock('@shikijs/magic-move/vue', () => ({
  ShikiMagicMovePrecompiled: { name: 'ShikiMagicMovePrecompiled', template: '<pre class="mm" />' }
}))

const fakeSandbox = (formatted: string): ProseSandbox => ({
  diagnostics : ref([]),
  error       : ref(''),
  formatted   : ref(formatted),
  source      : ref(formatted)
} as unknown as ProseSandbox)

const LintFlagPopperStub = { methods: { hide() {}, show() {} }, template: '<div class="popper-stub" />' }

const mountSurface = (sandbox: ProseSandbox) =>
  mount(ProseSandboxSurface, {
    props  : { sandbox },
    global : { stubs: { LintFlagPopper: LintFlagPopperStub } }
  })

describe('ProseSandboxSurface', () => {
  domTest('renders the formatted output and clicks into an editable draft', async ({ reducedMotion }) => {
    reducedMotion(false)
    const sandbox = fakeSandbox('x = 1')
    const wrapper = mountSurface(sandbox)
    await flushPromises()

    const display = wrapper.get('.sandbox-surface-display')
    expect(display.html()).toContain('x = 1')

    await display.trigger('click')
    await flushPromises()
    expect(wrapper.get('textarea').element.value).toBe('x = 1')
  })

  domTest('keeps reacting after a config-driven reformat', async ({ reducedMotion }) => {
    reducedMotion(false)
    const sandbox = fakeSandbox('x = 1')
    const wrapper = mountSurface(sandbox)
    await flushPromises()

    sandbox.formatted.value = 'x      = 1'
    await flushPromises()
    await nextFrame()
    await nextFrame()
    await flushPromises()

    // The settled html lands only after the morph mounts and paints, so the
    // display assert waits out the two frames `nextPaint` spans.
    sandbox.formatted.value = 'x = 2'
    await flushPromises()
    await nextFrame()
    await nextFrame()
    await flushPromises()
    expect(wrapper.get('.sandbox-surface-display').html()).toContain('x = 2')
  })

  domTest('drops a disabled rule\'s squiggle while keeping the code', async ({ reducedMotion }) => {
    reducedMotion(false)
    const sandbox = fakeSandbox('x = 1')
    sandbox.diagnostics.value = [
      { code: 'r1', end_location: { column: 2, row: 1 }, location: { column: 1, row: 1 }, message: 'm' }
    ]
    const wrapper = mountSurface(sandbox)
    await flushPromises()
    expect(wrapper.get('.sandbox-surface-display').html()).toContain('data-rule="r1"')

    sandbox.diagnostics.value = []
    await flushPromises()
    await drawSettled()
    await flushPromises()
    const display = wrapper.get('.sandbox-surface-display')
    expect(display.html()).not.toContain('data-rule="r1"')
    expect(display.html()).toContain('x = 1')
  })

  domTest('draws a newly enabled rule\'s squiggle onto unchanged code', async ({ reducedMotion }) => {
    reducedMotion(false)
    const sandbox = fakeSandbox('x = 1')
    const wrapper = mountSurface(sandbox)
    await flushPromises()
    expect(wrapper.get('.sandbox-surface-display').html()).not.toContain('data-rule="r1"')

    sandbox.diagnostics.value = [
      { code: 'r1', end_location: { column: 2, row: 1 }, location: { column: 1, row: 1 }, message: 'm' }
    ]
    await flushPromises()
    await drawSettled()
    await flushPromises()
    expect(wrapper.get('.sandbox-surface-display').html()).toContain('data-rule="r1"')
  })
})
