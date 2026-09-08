import { watchDebounced } from '@vueuse/core'
import { parse }          from 'smol-toml'
import { ref, toRaw }     from 'vue'

import type * as configSchema from '../sandbox/config-schema.data'
import { errorMessage }       from '../shared/error-message'
import { tomlText }           from '../shared/toml-text'

type FacetValue = configSchema.FacetValue
type ParsedRule = boolean | Record<string, FacetValue>

type ParsedConfig = { rules?: Record<string, ParsedRule> } &
  Record<string, number | Record<string, ParsedRule> | undefined>

// `structuredClone` rejects a reactive proxy, so unwrap to the raw object first.
function clone(config: ParsedConfig): ParsedConfig {
  return structuredClone(toRaw(config))
}

function parseToml(text: string): ParsedConfig {
  return text.trim() ? (parse(text) as ParsedConfig) : {}
}

// The `prose.toml` model behind the chip panel and the ruler, holding the text
// the editor binds and the parsed table the knobs read. `onToggle` fires for a
// discrete on/off action, which reformats without waiting out the typing
// debounce.
export function useSandboxConfig(
  schema     : configSchema.SandboxSchema,
  debounceMs : number,
  onToggle   : () => void
) {
  const configError = ref('')
  const configToml  = ref('')
  const parsed      = ref<ParsedConfig>({})

  function commit(next: ParsedConfig): void {
    if (next.rules && Object.keys(next.rules).length === 0) delete next.rules
    parsed.value      = next
    configToml.value  = tomlText(next)
    configError.value = ''
  }

  function defaultLength(key: string): number {
    return schema.lengths.find(knob => knob.key === key)?.default ?? schema.codeLineLength
  }

  // Restores a saved or shared config, where an unparseable table falls back to
  // the defaults rather than stranding the panel.
  function adopt(text: string): void {
    configToml.value = text
    try {
      parsed.value = parseToml(text)
    } catch {
      parsed.value = {}
    }
  }

  function facetValue(slug: string, facet: configSchema.Facet): FacetValue {
    const rule = parsed.value.rules?.[slug]
    if (facet.key === 'enabled') {
      if (rule === false) return false
      return typeof rule === 'object' ? rule.enabled ?? true : true
    }
    return typeof rule === 'object' ? rule[facet.key] ?? facet.default : facet.default
  }

  function lengthValue(key: string): number {
    const value = parsed.value[key]
    return typeof value === 'number' ? value : defaultLength(key)
  }

  // Renames the key at a dotted `path`, keeping the value it held. The rename
  // runs through the same parse and re-serialize that a knob change runs
  // through.
  function renameKey(path: string, to: string): void {
    const next     = clone(parsed.value) as Record<string, unknown>
    const segments = path.split('.')
    const leaf     = segments.pop() ?? ''
    const table    = segments.reduce<Record<string, unknown> | undefined>(
      (node, segment) => {
        const child = node?.[segment]
        return typeof child === 'object' && child !== null ? child as Record<string, unknown> : undefined
      },
      next
    )
    if (!table || !(leaf in table)) return
    table[to] = table[leaf]
    delete table[leaf]
    commit(next as ParsedConfig)
    onToggle()
  }

  function reset(): void {
    parsed.value     = {}
    configToml.value = ''
  }

  function setFacet(slug: string, facet: configSchema.Facet, value: FacetValue): void {
    const next    = clone(parsed.value)
    const rules   = (next.rules ??= {})
    const current = rules[slug]
    if (facet.key === 'enabled') {
      if (value === false) rules[slug] = false
      else if (current === false) delete rules[slug]
      else if (typeof current === 'object') delete current.enabled
      commit(next)
      onToggle()
      return
    }
    const table = typeof current === 'object' ? current : {}
    if (value === facet.default) delete table[facet.key]
    else table[facet.key] = value
    if (Object.keys(table).length === 0) delete rules[slug]
    else rules[slug] = table
    commit(next)
    if (facet.kind === 'bool') onToggle()
  }

  function setLength(key: string, value: number): void {
    const next = clone(parsed.value)
    if (value === defaultLength(key)) delete next[key]
    else next[key] = value
    commit(next)
  }

  watchDebounced(configToml, text => {
    try {
      parsed.value      = parseToml(text)
      configError.value = ''
    } catch (thrown) {
      configError.value = String(errorMessage(thrown))
    }
  }, { debounce: debounceMs })

  return {
    adopt, configError, configToml, facetValue, lengthValue, renameKey, reset, setFacet, setLength
  }
}
