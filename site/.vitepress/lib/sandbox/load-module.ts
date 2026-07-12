// The slice of the `--target web` glue the sandbox calls: the default init
// and the `format` entry point returning the rewritten source, the effective
// config, and the lint findings as a JSON records string.
export interface ProseWasm {
  default : (init ?: unknown) => Promise<unknown>
  format  : (configToml: string, source: string) => {
    config      : string
    diagnostics : string
    fired_rules : string
    formatted   : string
  }
}

// A cache-busting query hands each recovery a fresh module namespace, because
// the glue caches its instance behind `if (wasm !== undefined)` and a trapped
// instance stays poisoned. Relative resolution drops the query from the binary
// URL, so the module reloads from the HTTP cache rather than the network.
export function loadModule(reinit: number): Promise<ProseWasm> {
  const bust = reinit > 0 ? `?reinit=${reinit}` : ''
  // oxlint-disable-next-line no-inline-comments -- @vite-ignore must sit inside import()
  return import(/* @vite-ignore */ `/wasm/prose_wasm.js${bust}`) as Promise<ProseWasm>
}
