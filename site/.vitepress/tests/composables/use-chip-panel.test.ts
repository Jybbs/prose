// @vitest-environment happy-dom
import { ref } from 'vue'

import { useChipPanel }              from '../../lib/composables/use-chip-panel'
import type { Facet, RuleControl }   from '../../lib/composables/use-chip-panel'
import type { ProseSandbox }         from '../../lib/composables/use-prose-sandbox'
import type { RenderedRule }         from '../../lib/rules/rules.data'
import { mountSetup }                from '../dom'

const ALIGN: RuleControl = {
  family : 'alignment',
  slug   : 'align-equals',
  facets : [
    { default: true, hintHtml: '', key: 'enabled', kind: 'bool', label: 'Enabled' },
    { default: 16, hintHtml: 'The width-spread budget.', key: 'max-shift', kind: 'int', label: 'Max Shift' },
    { default: true, hintHtml: '', key: 'condense', kind: 'bool', label: 'Condense' }
  ]
}

const BLANK: RuleControl = {
  family : 'formatting',
  slug   : 'blank-lines',
  facets : [{ default: 2, hintHtml: '', key: 'gap', kind: 'int', label: 'Gap' }]
}

const CARDS = {
  'align-equals': { href: '/rules/alignment/align-equals', slug: 'align-equals' } as RenderedRule
}

// A stateful stand-in for the sandbox: `setFacet` writes into a plain map the
// `facetValue` reads resolve against, which is all the panel logic touches.
function fakeSandbox() {
  const overrides = new Map<string, unknown>()
  const eligible     = ref<readonly string[] | null>([])
  const facetImpact  = ref<Record<string, readonly string[]>>({})
  const lengthImpact = ref<readonly string[] | null>(null)
  const sandbox = {
    eligible,
    facetImpact,
    facetValue : (slug: string, facet: Facet) => overrides.get(`${slug}.${facet.key}`) ?? facet.default,
    lengthImpact,
    lengths    : [
      { default: 88, key: 'code-line-length', label: 'Code' },
      { default: 76, key: 'docstring-line-length', label: 'Docstring' }
    ],
    lengthValue: () => 88,
    rules      : [ALIGN, BLANK],
    setFacet   : (slug: string, facet: Facet, value: unknown) => {
      overrides.set(`${slug}.${facet.key}`, value)
    },
    setLength  : () => {}
  } as unknown as ProseSandbox
  return { eligible, facetImpact, lengthImpact, sandbox }
}

describe('useChipPanel', () => {
  it('filters the visible rules to the eligible set and falls back to all', () => {
    const { eligible, sandbox } = fakeSandbox()
    const api = mountSetup(() => useChipPanel(sandbox, CARDS))
    expect(api.visible.value.map(rule => rule.slug)).toEqual(['align-equals', 'blank-lines'])
    eligible.value = ['blank-lines']
    expect(api.visible.value.map(rule => rule.slug)).toEqual(['blank-lines'])
    eligible.value = null
    expect(api.visible.value).toEqual([])
  })

  it('reads and toggles a rule through its enabled facet', () => {
    const { sandbox } = fakeSandbox()
    const api = mountSetup(() => useChipPanel(sandbox, CARDS))
    expect(api.isOn(ALIGN)).toBe(true)
    api.toggle(ALIGN)
    expect(api.isOn(ALIGN)).toBe(false)
  })

  it('closes the open facet surface when its rule toggles off', () => {
    const { sandbox } = fakeSandbox()
    const api = mountSetup(() => useChipPanel(sandbox, CARDS))
    api.openFacets(ALIGN)
    expect(api.openSlug.value).toBe('align-equals')
    api.toggle(ALIGN)
    expect(api.openSlug.value).toBe('')
  })

  it('ignores a facet-surface open on a disabled rule and toggles otherwise', () => {
    const { sandbox } = fakeSandbox()
    const api = mountSetup(() => useChipPanel(sandbox, CARDS))
    api.toggle(ALIGN)
    api.openFacets(ALIGN)
    expect(api.openSlug.value).toBe('')
    api.toggle(ALIGN)
    api.openFacets(ALIGN)
    api.openFacets(ALIGN)
    expect(api.openSlug.value).toBe('')
  })

  it('falls back to the first facet when a rule carries no enabled key', () => {
    const { sandbox } = fakeSandbox()
    const api = mountSetup(() => useChipPanel(sandbox, CARDS))
    expect(api.enabledFacet(BLANK).key).toBe('gap')
  })

  it('narrows the sub-facets to the probed impact set and hides them unprobed', () => {
    const { facetImpact, sandbox } = fakeSandbox()
    const api = mountSetup(() => useChipPanel(sandbox, CARDS))
    expect(api.subFacets(ALIGN)).toEqual([])
    facetImpact.value = { 'align-equals': ['max-shift', 'condense'] }
    expect(api.subFacets(ALIGN).map(facet => facet.key)).toEqual(['max-shift', 'condense'])
    facetImpact.value = { 'align-equals': ['condense'] }
    expect(api.subFacets(ALIGN).map(facet => facet.key)).toEqual(['condense'])
  })

  it('narrows the ruler knobs to the probed impact set and hides them unprobed', () => {
    const { lengthImpact, sandbox } = fakeSandbox()
    const api = mountSetup(() => useChipPanel(sandbox, CARDS))
    expect(api.visibleLengths.value).toEqual([])
    lengthImpact.value = ['docstring-line-length']
    expect(api.visibleLengths.value.map(knob => knob.key)).toEqual(['docstring-line-length'])
  })

  it('resolves a rendered rule card and misses null-safely', () => {
    const { sandbox } = fakeSandbox()
    const api = mountSetup(() => useChipPanel(sandbox, CARDS))
    expect(api.ruleData('align-equals')?.href).toBe('/rules/alignment/align-equals')
    expect(api.ruleData('blank-lines')).toBeNull()
  })

  it('closes on an outside click but leaves a gear click to its own handler', async () => {
    const { sandbox } = fakeSandbox()
    const api = mountSetup(() => useChipPanel(sandbox, CARDS))
    const panel = document.createElement('div')
    const gear  = document.createElement('button')
    const away  = document.createElement('div')
    gear.dataset.gear = ''
    document.body.append(panel, gear, away)
    api.setPanel(panel)
    api.openSlug.value = 'align-equals'
    const press = (el: HTMLElement) => {
      el.dispatchEvent(new MouseEvent('pointerdown', { bubbles: true }))
      el.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    }
    press(gear)
    expect(api.openSlug.value).toBe('align-equals')
    await new Promise(resolve => { setTimeout(resolve, 0) })
    press(away)
    expect(api.openSlug.value).toBe('')
    panel.remove()
    gear.remove()
    away.remove()
  })
})
