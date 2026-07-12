import { stringify } from 'smol-toml'

import type { Facet, FacetValue } from './config-schema.data'
import type { ProseWasm }         from './load-module'

const INT_PROBES    = [1, 500] as const
const LENGTH_PROBES = [30, 180] as const

export type ProbeBaseline = { diagnostics: string, formatted: string }

// The probe values that could reveal a facet's effect on a source: a bool
// flips its default and an int takes each extreme, whereas a string kind has
// no finite probe set, so `null` marks it unprobeable.
function facetProbes(facet: Facet): readonly FacetValue[] | null {
  if (facet.kind === 'bool') return [facet.default !== true]
  if (facet.kind === 'int') return INT_PROBES
  return null
}

// A probe set moves a source when any probed config changes the output or
// findings against the default-run baseline. A failed run proves nothing,
// so it counts as a move, keeping the probed knob visible.
function probesDiffer(
  baseline : ProbeBaseline,
  format   : ProseWasm['format'],
  probes   : readonly FacetValue[],
  source   : string,
  toConfig : (value: FacetValue) => object
): boolean {
  return probes.some(value => {
    try {
      const run = format(stringify(toConfig(value)), source)
      return run.formatted !== baseline.formatted || run.diagnostics !== baseline.diagnostics
    } catch {
      return true
    }
  })
}

// A facet has impact when any probe run differs from the default-run
// baseline. An unprobeable facet counts as impact, leaving it visible.
export function facetHasImpact(
  baseline : ProbeBaseline,
  facet    : Facet,
  format   : ProseWasm['format'],
  slug     : string,
  source   : string
): boolean {
  const probes = facetProbes(facet)
  if (!probes) return true
  return probesDiffer(baseline, format, probes, source, value => ({
    rules: { [slug]: { [facet.key]: value } }
  }))
}

// A length knob has impact when either extreme changes the output against
// the default-run baseline.
export function lengthHasImpact(
  baseline : ProbeBaseline,
  format   : ProseWasm['format'],
  key      : string,
  source   : string
): boolean {
  return probesDiffer(baseline, format, LENGTH_PROBES, source, value => ({ [key]: value }))
}
