import { onClickOutside } from '@vueuse/core'
import { computed, ref }  from 'vue'

import type { RenderedRule }       from '../rules/rules.data'
import type { Facet, RuleControl } from '../sandbox/config-schema.data'
import type { ProseSandbox }       from './use-prose-sandbox'

// The shared chip-panel view over one sandbox: eligibility-filtered rules and
// the on/off toggles plus the facet-surface open state. The panel component
// renders the cosmetics over this logic and passes the rendered rule cards
// in, keeping the composable free of the build-time loader.
export function useChipPanel(sandbox: ProseSandbox, cards: Record<string, RenderedRule>) {
  const {
    eligible, facetImpact, facetValue, lengthImpact, lengthValue, lengths,
    rules: controls, setFacet, setLength
  } = sandbox

  const openSlug = ref('')
  const panel    = ref<HTMLElement | null>(null)

  // An unprobed source (`null`) renders no tags rather than flashing the full
  // roster, whereas a probed source with nothing fired falls back to all.
  const visible = computed(() => {
    const fired = eligible.value
    if (!fired) return []
    const eligibleSet = new Set(fired)
    return fired.length > 0 ? controls.filter(rule => eligibleSet.has(rule.slug)) : controls
  })

  // A probed source narrows the ruler to the knobs that can affect it, and
  // an unprobed source (`null`) renders none until the probe lands.
  const visibleLengths = computed(() => {
    const impact = lengthImpact.value
    return impact ? lengths.filter(knob => impact.includes(knob.key)) : []
  })

  function ruleData(slug: string): RenderedRule | null {
    return cards[slug] ?? null
  }

  function enabledFacet(rule: RuleControl): Facet {
    return rule.facets.find(facet => facet.key === 'enabled') ?? rule.facets[0]
  }

  // A probed rule narrows to the facets that can affect the current source.
  // A slug the probe map does not carry yet shows none, so gears appear as
  // probes land rather than flashing and vanishing.
  function subFacets(rule: RuleControl): readonly Facet[] {
    const impact = facetImpact.value[rule.slug]
    if (!impact) return []
    return rule.facets.filter(facet => facet.key !== 'enabled' && impact.includes(facet.key))
  }

  function isOn(rule: RuleControl): boolean {
    return facetValue(rule.slug, enabledFacet(rule)) === true
  }

  function toggle(rule: RuleControl): void {
    const next = !isOn(rule)
    setFacet(rule.slug, enabledFacet(rule), next)
    if (!next && openSlug.value === rule.slug) openSlug.value = ''
  }

  function openFacets(rule: RuleControl): void {
    if (!isOn(rule)) return
    openSlug.value = openSlug.value === rule.slug ? '' : rule.slug
  }

  function setPanel(el: unknown): void {
    panel.value = el as HTMLElement | null
  }

  // A gear click toggles its own popover, so it stays with that handler
  // rather than closing the plate first.
  onClickOutside(panel, () => { openSlug.value = '' }, { ignore: ['[data-gear]'] })

  return {
    enabledFacet,
    isOn,
    lengthValue,
    openFacets,
    openSlug,
    ruleData,
    setLength,
    setPanel,
    subFacets,
    toggle,
    visible,
    visibleLengths
  }
}

export type { Facet, RuleControl }
