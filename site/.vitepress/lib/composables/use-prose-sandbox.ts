import { watchDebounced } from '@vueuse/core'
import { ref, type Ref }  from 'vue'

import { loadModule, type ProseWasm } from '../sandbox/load-module'
import { errorMessage }               from '../shared/error-message'

export interface ProseSandbox {
  error  : Ref<string>
  format : () => Promise<void>
  output : Ref<string>
  source : Ref<string>
  status : Ref<SandboxStatus>
}

export interface ProseSandboxOptions {
  debounceMs ?: number
  load       ?: (reinit: number) => Promise<ProseWasm>
  source      : string
}

type SandboxStatus = 'idle' | 'loading'

const TRAP_NOTICE =
  'The formatter hit an internal error on this input. Edit the source to try again.'

export function useProseSandbox(options: ProseSandboxOptions): ProseSandbox {
  const { debounceMs = 160, load = loadModule, source: seed } = options

  const error  = ref('')
  const output = ref('')
  const source = ref(seed)
  const status = ref<SandboxStatus>('idle')

  let module: ProseWasm | null = null
  let reinit = 0

  async function instantiate(): Promise<ProseWasm> {
    const next = await load(reinit)
    await next.default()
    return next
  }

  async function format(): Promise<void> {
    if (!module) status.value = 'loading'
    try {
      module    ??= await instantiate()
      output.value = module.format('', source.value).formatted
      error.value  = ''
    } catch (thrown) {
      if (thrown instanceof WebAssembly.RuntimeError) {
        // A panic poisons the instance, so drop it and bump the counter,
        // leaving the next format to instantiate a fresh module.
        module = null
        reinit += 1
        error.value = TRAP_NOTICE
      } else {
        error.value = String(errorMessage(thrown))
      }
    } finally {
      status.value = 'idle'
    }
  }

  watchDebounced(source, format, { debounce: debounceMs })
  return { error, format, output, source, status }
}
