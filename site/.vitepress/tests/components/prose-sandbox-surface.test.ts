// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils'
import { promiseTimeout }       from '@vueuse/core'
import { ref }                  from 'vue'

import ProseSandboxSurface             from '../../theme/components/sandbox/ProseSandboxSurface.vue'
import type { ProseSandbox }           from '../../lib/composables/use-prose-sandbox'
import { nextPaint }                   from '../../lib/shared/paint'
import { domTest, isHidden, stubRect } from '../dom'

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
  source      : ref(source),
  unstable    : ref([])
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

interface MountOptions {
  diagnostics ?: ProseSandbox['diagnostics']['value']
  formatted    : string
  motion      ?: boolean
  source      ?: string
}

const surfaceMount = (reducedMotion: (matches: boolean) => void) =>
  async (options: MountOptions) => {
    const { diagnostics, formatted, motion = false, source = formatted } = options
    reducedMotion(motion)
    const sandbox = fakeSandbox(formatted, source)
    if (diagnostics) sandbox.diagnostics.value = diagnostics
    const wrapper = mount(ProseSandboxSurface, {
      props  : { sandbox },
      global : { stubs: { LintFlagPopper: LintFlagPopperStub } }
    })
    await flushPromises()
    return { sandbox, wrapper }
  }

const surfaceTest = domTest.extend<{ mounted: ReturnType<typeof surfaceMount> }>({
  mounted: async ({ reducedMotion }, use) => { await use(surfaceMount(reducedMotion)) }
})

describe('ProseSandboxSurface', () => {
  surfaceTest('opens the editor on the source rather than the formatted output', async ({ mounted }) => {
    const { wrapper } = await mounted({ formatted: 'x = 1', source: 'x=1' })

    const display = wrapper.get('.sandbox-surface-display')
    expect(display.html()).toContain('x = 1')

    // Seeding from the formatted output would feed an already-formatted source
    // back in, leaving every rewriting rule with nothing left to do.
    await display.trigger('click')
    await flushPromises()
    expect(wrapper.get('textarea').element.value).toBe('x=1')
  })

  surfaceTest('leaves the source alone until the reader applies the edit', async ({ mounted }) => {
    const { sandbox, wrapper } = await mounted({ formatted: 'x = 1', source: 'x=1' })

    await wrapper.get('.sandbox-surface-display').trigger('click')
    await flushPromises()
    await wrapper.get('textarea').setValue('y=2')
    await wrapper.get('textarea').trigger('blur')
    expect(sandbox.source.value).toBe('x=1')

    await wrapper.get('.sandbox-surface-apply').trigger('click')
    expect(sandbox.source.value).toBe('y=2')
    expect(isHidden(wrapper.get('.code-editor'))).toBe(true)
  })

  surfaceTest('discards the edit and keeps the source', async ({ mounted }) => {
    const { sandbox, wrapper } = await mounted({ formatted: 'x = 1', source: 'x=1' })

    await wrapper.get('.sandbox-surface-display').trigger('click')
    await flushPromises()
    await wrapper.get('textarea').setValue('y=2')
    await wrapper.get('.sandbox-surface-discard').trigger('click')

    expect(sandbox.source.value).toBe('x=1')
    expect(isHidden(wrapper.get('.code-editor'))).toBe(true)
  })

  surfaceTest('drives the apply-pane from the keyboard', async ({ mounted }) => {
    const { sandbox, wrapper } = await mounted({ formatted: 'x = 1', source: 'x=1' })

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

  surfaceTest('keeps reacting after a config-driven reformat', async ({ mounted }) => {
    const { sandbox, wrapper } = await mounted({ formatted: 'x = 1' })

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

  surfaceTest('holds the outgoing height until the morph flips its step', async ({ mounted }) => {
    const { sandbox, wrapper } = await mounted({ formatted: 'x = 1' })
    stubRect(wrapper.element, { height: 320.4 })

    sandbox.formatted.value = 'x = 2'
    await flushPromises()
    expect(wrapper.attributes('style')).toContain('min-height: 321px')

    // The container's own height animation carries the panel from the flip on.
    await nextPaint()
    await flushPromises()
    expect(wrapper.attributes('style')).toBeUndefined()
  })

  surfaceTest('clears the morph when a reverting render supersedes it', async ({ mounted }) => {
    const { sandbox, wrapper } = await mounted({ formatted: 'x = 1' })

    sandbox.formatted.value = 'x = 2'
    await flushPromises()
    expect(wrapper.find('.mm').exists()).toBe(true)

    sandbox.formatted.value = 'x = 1'
    await flushPromises()
    expect(isHidden(wrapper.get('.mm'))).toBe(true)
    expect(isHidden(wrapper.get('.sandbox-surface-display'))).toBe(false)
    expect(wrapper.attributes('style')).toBeUndefined()
  })

  surfaceTest('skips the morph and its hold under reduced motion', async ({ mounted }) => {
    const { sandbox, wrapper } = await mounted({ formatted: 'x = 1', motion: true })

    sandbox.formatted.value = 'x = 2'
    await flushPromises()
    expect(wrapper.find('.mm').exists()).toBe(false)
    expect(wrapper.attributes('style')).toBeUndefined()
    expect(wrapper.get('.sandbox-surface-display').html()).toContain('x = 2')
  })

  surfaceTest('releases the hold when the reader opens the editor mid-morph', async ({ mounted }) => {
    const { sandbox, wrapper } = await mounted({ formatted: 'x = 1' })
    stubRect(wrapper.element, { height: 260 })

    sandbox.formatted.value = 'x = 2'
    await flushPromises()
    expect(wrapper.attributes('style')).toContain('min-height: 260px')

    await wrapper.get('.sandbox-surface-display').trigger('click')
    await flushPromises()
    expect(wrapper.attributes('style')).toBeUndefined()
    expect(isHidden(wrapper.get('.mm'))).toBe(true)
  })

  surfaceTest('clears a running morph when the parent opens the editor', async ({ mounted }) => {
    const { sandbox, wrapper } = await mounted({ formatted: 'x = 1' })

    sandbox.formatted.value = 'x = 2'
    await flushPromises()
    expect(wrapper.find('.mm').exists()).toBe(true)

    // The parent owns `editing`, so it opens the editor without the click path
    // that clears the morph on its way in.
    await wrapper.setProps({ editing: true })
    sandbox.formatted.value = 'x = 3'
    await flushPromises()
    expect(isHidden(wrapper.get('.mm'))).toBe(true)
  })

  surfaceTest('keeps one panel mounted across consecutive morphs', async ({ mounted }) => {
    const { sandbox, wrapper } = await mounted({ formatted: 'x = 1' })

    sandbox.formatted.value = 'x = 2'
    await flushPromises()
    const element = wrapper.get('.mm').element
    await nextPaint()
    await flushPromises()

    // The second morph reuses the same panel DOM rather than remounting.
    sandbox.formatted.value = 'x = 3'
    await flushPromises()
    expect(wrapper.get('.mm').element).toBe(element)
    expect(isHidden(wrapper.get('.mm'))).toBe(false)
  })

  surfaceTest('ignores a superseded morph\'s stale end and settles last', async ({ mounted }) => {
    const { sandbox, wrapper } = await mounted({ formatted: 'x = 1' })

    sandbox.formatted.value = 'x = 2'
    await flushPromises()
    await nextPaint()
    await flushPromises()

    sandbox.formatted.value = 'x = 3'
    await flushPromises()
    await nextPaint()
    await flushPromises()

    // The first morph's force-resolved end lands on the live panel, so the
    // successor keeps animating until its own end arrives.
    const panel = wrapper.findComponent({ name: 'ShikiMagicMovePrecompiled' })
    panel.vm.$emit('end')
    await flushPromises()
    expect(isHidden(wrapper.get('.mm'))).toBe(false)

    panel.vm.$emit('end')
    await flushPromises()
    expect(isHidden(wrapper.get('.mm'))).toBe(true)
    expect(isHidden(wrapper.get('.sandbox-surface-display'))).toBe(false)
  })

  surfaceTest('absorbs a stray end and still settles the next morph', async ({ mounted }) => {
    const { sandbox, wrapper } = await mounted({ formatted: 'x = 1' })
    const panel = () => wrapper.findComponent({ name: 'ShikiMagicMovePrecompiled' })

    sandbox.formatted.value = 'x = 2'
    await flushPromises()
    await nextPaint()
    await flushPromises()
    panel().vm.$emit('end')
    await flushPromises()
    expect(isHidden(wrapper.get('.mm'))).toBe(true)

    // A settled surface still taking an `end` must not drive the count
    // negative, which would leave the next morph unable to reach zero.
    panel().vm.$emit('end')
    await flushPromises()

    sandbox.formatted.value = 'x = 3'
    await flushPromises()
    await nextPaint()
    await flushPromises()
    panel().vm.$emit('end')
    await flushPromises()
    expect(isHidden(wrapper.get('.mm'))).toBe(true)
  })

  surfaceTest('settles the surface when no end ever arrives', async ({ mounted }) => {
    const { sandbox, wrapper } = await mounted({ formatted: 'x = 1' })

    sandbox.formatted.value = 'x = 2'
    await flushPromises()
    await nextPaint()
    await flushPromises()
    expect(isHidden(wrapper.get('.mm'))).toBe(false)

    // The stub panel never emits `end`, so only the watchdog can settle this.
    await promiseTimeout(800)
    await flushPromises()
    expect(isHidden(wrapper.get('.mm'))).toBe(true)
    expect(isHidden(wrapper.get('.sandbox-surface-display'))).toBe(false)
  })

  surfaceTest('re-measures the outgoing height on every morph', async ({ mounted }) => {
    const { sandbox, wrapper } = await mounted({ formatted: 'x = 1' })
    stubRect(wrapper.element, { height: 200 })

    sandbox.formatted.value = 'x = 2'
    await flushPromises()
    expect(wrapper.attributes('style')).toContain('min-height: 200px')
    await nextPaint()
    await flushPromises()

    stubRect(wrapper.element, { height: 480 })
    sandbox.formatted.value = 'x = 3'
    await flushPromises()
    expect(wrapper.attributes('style')).toContain('min-height: 480px')
  })

  surfaceTest('drops a disabled rule\'s squiggle while keeping the code', async ({ mounted }) => {
    const { sandbox, wrapper } = await mounted({
      diagnostics : [FINDING],
      formatted   : 'x = 1'
    })
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

  surfaceTest('draws a newly enabled rule\'s squiggle onto unchanged code', async ({ mounted }) => {
    const { sandbox, wrapper } = await mounted({ formatted: 'x = 1' })
    expect(wrapper.get('.sandbox-surface-display').html()).not.toContain('data-rule="r1"')

    sandbox.diagnostics.value = [FINDING]
    await flushPromises()
    await drawSettled()
    await flushPromises()

    // The freshly enabled underline draws back in rather than staying staged.
    expect(wrapper.get('.lint-flag[data-rule="r1"]').classes()).not.toContain('lint-undrawn')
  })

  surfaceTest('washes a whole-row finding rather than underlining it', async ({ mounted }) => {
    const { wrapper } = await mounted({
      diagnostics : [{ ...FINDING, end_location: { column: 6, row: 1 } }],
      formatted   : 'x = 1',
      motion      : true
    })

    expect(wrapper.get('.lint-flag[data-rule="r1"]').classes()).toContain('lint-flag-line')
  })
})
