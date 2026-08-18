import { promiseTimeout } from '@vueuse/core'
import { ref }            from 'vue'

import type * as configSchema from '../sandbox/config-schema.data'
import type { ProseWasm }     from '../sandbox/load-module'
import * as probe             from '../sandbox/probe'
import { nextPaint }          from '../shared/paint'

type SourceProbe = {
  eligible     : readonly string[]
  facetImpact  : Record<string, readonly string[]>
  lengthImpact : readonly string[]
}

const EMPTY_PROBE: SourceProbe = { eligible: [], facetImpact: {}, lengthImpact: [] }

// The rules that fire under the default config are the ones that can affect a
// snippet, and each eligible rule's sub-facets are probed against that
// default-run baseline so the panel can hide the knobs that cannot affect it.
// `isCurrent` reports whether the module a run started on is still the live
// one, so a run on an instance a panic has since replaced does not seed the
// cache.
export function useSandboxProbe(
  schema    : configSchema.SandboxSchema,
  isCurrent : (module: ProseWasm) => boolean
) {
  const eligible     = ref<readonly string[] | null>(null)
  const facetImpact  = ref<Record<string, readonly string[]>>({})
  const lengthImpact = ref<readonly string[] | null>(null)

  const probed = new Map<string, SourceProbe>()

  let probedSource = '\0'

  // A null result is the unprobed sentinel the panel renders as "still
  // probing", so the three refs always move together.
  function apply(result: SourceProbe | null): void {
    eligible.value     = result?.eligible     ?? null
    facetImpact.value  = result?.facetImpact  ?? {}
    lengthImpact.value = result?.lengthImpact ?? null
  }

  // Probes a source in the background, where the default run seeds the eligible
  // set before the first await, then each eligible rule's sub-facets are probed
  // against that baseline one rule per macrotask, so the burst of probe runs
  // never blocks a display format. A source change mid-loop abandons the stale
  // loop.
  async function run(current: ProseWasm, target: string): Promise<void> {
    let baseline: ReturnType<ProseWasm['format']>
    try {
      baseline = current.format('', target)
    } catch {
      // A failed probe yields no impact data, so every sentinel settles together
      // rather than leaving the ruler reading an unprobed source forever.
      apply(EMPTY_PROBE)
      return
    }
    const fired = baseline.fired_rules
    eligible.value = fired
    await promiseTimeout(0)
    if (probedSource !== target) return
    const lengths = schema.lengths
      .filter(knob => probe.lengthHasImpact(baseline, current.format, knob.key, target))
      .map(knob => knob.key)
    lengthImpact.value = lengths
    const impact: Record<string, readonly string[]> = {}
    for (const rule of schema.rules) {
      if (!fired.includes(rule.slug)) continue
      await promiseTimeout(0)
      if (probedSource !== target) return
      impact[rule.slug] = rule.facets
        .filter(facet =>
          facet.key !== 'enabled' &&
          probe.facetHasImpact(baseline, facet, current.format, rule.slug, target))
        .map(facet => facet.key)
      facetImpact.value = { ...impact }
    }
    if (isCurrent(current)) {
      probed.set(target, { eligible: fired, facetImpact: impact, lengthImpact: lengths })
    }
  }

  // Adopts the target's cached probe or kicks a fresh run, two paints after
  // the caller's publish, because the plate deck the adoption rebuilds would
  // otherwise share the publish frame and stretch it.
  async function adopt(current: ProseWasm, target: string): Promise<void> {
    await nextPaint()
    await nextPaint()
    if (probedSource !== target) return
    const cached = probed.get(target)
    apply(cached ?? null)
    if (!cached) void run(current, target)
  }

  // Called after each successful format. A config toggle never changes what is
  // eligible, so only a new source adopts its cached probe or kicks a fresh run.
  function sync(current: ProseWasm, target: string): void {
    if (target === probedSource) return
    probedSource = target
    void adopt(current, target)
  }

  return { eligible, facetImpact, lengthImpact, sync }
}
