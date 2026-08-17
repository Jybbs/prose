import type { KeyedTokensInfo }      from '@shikijs/magic-move/types'
import { computed, ref, shallowRef } from 'vue'

import { magicMoveOptions, type MagicMovePanel } from '../markdown/magic-move-options'
import { ruleDrawMs, ruleEasing }                from '../shared/paint'

// The precompiled panel re-syncs its keys, with in-place side effects,
// whenever these prop identities change, so they stay stable across
// unrelated re-renders instead of rebuilding per template pass.
export function useMagicMove(stagger?: number) {
  const duration = ref(0)
  const easing   = ref('')
  const panel    = shallowRef<MagicMovePanel>(null)
  const steps    = shallowRef<readonly KeyedTokensInfo[]>([])

  const morphOptions = computed(() => magicMoveOptions(duration.value, easing.value, stagger))
  const morphSteps   = computed(() => [...steps.value])

  // Resolves the renderer on first use and tokenizes the pair into the steps
  // the panel morphs between. The caller assigns `steps`, so a superseded run
  // drops its result rather than writing it.
  async function precompile(from: string, to: string): Promise<readonly KeyedTokensInfo[]> {
    const [{ precompileMagicMove }, { ShikiMagicMovePrecompiled }] = await Promise.all([
      import('../markdown/magic-move'),
      import('@shikijs/magic-move/vue')
    ])
    panel.value    = ShikiMagicMovePrecompiled
    duration.value = ruleDrawMs()
    easing.value   = ruleEasing()
    return precompileMagicMove([from, to])
  }

  return { duration, morphOptions, morphSteps, panel, precompile, steps }
}
