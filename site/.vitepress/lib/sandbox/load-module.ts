import type { LintFinding } from '../fixtures/lint-findings'

const MODULE_URL = '/wasm/prose_wasm.js'

// The record `format` returns, where `config_notices` carries one line per
// config key prose did not recognize, `fired_rules` the slugs of the rules
// that edited, and `unstable_rules` the slugs a second run would still edit.
export interface ProseFormat {
  config_notices : readonly string[]
  diagnostics    : readonly LintFinding[]
  fired_rules    : readonly string[]
  formatted      : string
  unstable_rules : readonly string[]
}

// The part of the `--target web` glue the sandbox calls. Passing `settle` as
// false skips the second pass over the output and leaves `unstable_rules`
// empty, and `__wbg_reset_state` rebuilds a trapped instance in place.
export interface ProseWasm {
  __wbg_reset_state : () => void
  default           : (init ?: unknown) => Promise<unknown>
  format            : (configToml: string, source: string, settle: boolean) => ProseFormat
}

// The glue loads once per page, since a trapped instance recovers in place
// through `__wbg_reset_state`.
export function loadModule(): Promise<ProseWasm> {
  // oxlint-disable-next-line no-inline-comments -- @vite-ignore must sit inside import()
  return import(/* @vite-ignore */ MODULE_URL) as Promise<ProseWasm>
}
