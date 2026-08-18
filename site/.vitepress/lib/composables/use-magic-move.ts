import type { KeyedTokensInfo }      from '@shikijs/magic-move/types'
import { computed, ref, shallowRef } from 'vue'

import type { MagicMoveMachine }                 from '../markdown/magic-move'
import { lineChurn, type MorphMeasure }          from '../markdown/magic-move-delta'
import { magicMoveOptions, magicMoveWatchdogMs } from '../markdown/magic-move-options'
import type { MagicMovePanel }                   from '../markdown/magic-move-options'
import { ruleDrawMs }                            from '../shared/paint'

export interface MorphPair extends MorphMeasure {
  steps : readonly KeyedTokensInfo[]
}

// `syncTokenKeys` assigns `key` onto the token objects it is handed, and the
// panel runs its own pass over whatever `steps` carries, so a step copies out
// of the machine rather than sharing its retained state with the panel.
function detach(step: KeyedTokensInfo): KeyedTokensInfo {
  return { ...step, tokens: step.tokens.map(token => ({ ...token })) }
}

// The precompiled panel re-syncs its keys, with in-place side effects,
// whenever these prop identities change, so they stay stable across
// unrelated re-renders instead of rebuilding per template pass.
export function useMagicMove(stagger?: number) {
  const duration = ref(0)
  const panel    = shallowRef<MagicMovePanel>(null)
  const steps    = shallowRef<readonly KeyedTokensInfo[]>([])

  const morphOptions = computed(() => magicMoveOptions(duration.value, stagger))
  const morphSteps   = computed(() => [...steps.value])
  const watchdogMs   = computed(() => magicMoveWatchdogMs(duration.value))

  let committed = ''
  let machine: MagicMoveMachine | null = null
  let starting: Promise<MagicMoveMachine> | null = null

  // Resolves the renderer and the machine on first use, behind one promise so
  // concurrent callers share the fetch rather than racing two of them.
  function start(): Promise<MagicMoveMachine> {
    starting ??= (async () => {
      const [{ magicMoveMachine }, { ShikiMagicMovePrecompiled }] = await Promise.all([
        import('../markdown/magic-move'),
        import('@shikijs/magic-move/vue')
      ])
      panel.value    = ShikiMagicMovePrecompiled
      duration.value = ruleDrawMs()
      machine        = await magicMoveMachine()
      return machine
    })()
    return starting
  }

  // Tokenizes `to` into the pair the panel morphs between. The machine already
  // holds `from` whenever the caller is stepping forward from the state it last
  // committed, so the common toggle tokenizes once. A superseded morph leaves
  // the machine ahead of what the surface shows, so a `from` it does not hold
  // rebases it first. The caller assigns `steps`, so a superseded run drops its
  // result rather than writing it.
  async function precompile(from: string, to: string): Promise<MorphPair> {
    const active = machine ?? await start()
    if (committed !== from) {
      active.reset()
      active.commit(from)
    }
    const { current, previous } = active.commit(to)
    committed = to
    return { ...lineChurn(from, to), steps: [detach(previous), detach(current)] }
  }

  return { morphOptions, morphSteps, panel, precompile, steps, watchdogMs }
}
