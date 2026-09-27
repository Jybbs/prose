// @vitest-environment happy-dom
import { effectScope } from 'vue'

import { useSandboxProbe }    from '../../lib/composables/use-sandbox-probe'
import type { SandboxSchema } from '../../lib/sandbox/config-schema.data'
import type { ProseWasm }     from '../../lib/sandbox/load-module'
import { nextFrame }          from '../dom'

const SCHEMA: SandboxSchema = { codeLineLength: 88, lengths: [], rules: [] }

describe('useSandboxProbe', () => {
  it('abandons a pending adoption once its scope stops', async () => {
    const format = vi.fn<ProseWasm['format']>()
    const scope  = effectScope()
    const probe  = scope.run(() => useSandboxProbe(SCHEMA))!
    probe.sync({ format } as unknown as ProseWasm, 'x = 1\n')
    scope.stop()
    for (let frame = 0; frame < 6; frame++) await nextFrame()
    expect(format).not.toHaveBeenCalled()
  })
})
