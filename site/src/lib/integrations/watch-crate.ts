import type { AstroIntegration } from 'astro'

import { cargoTomlPath, proseBinaryCandidates, ruleSourcePath } from '../shared/paths'

// Watches the crate sources and the compiled binary the build-time loaders read,
// so a `cargo build` or a `rule.rs` edit refreshes the generated collections in
// dev without restarting the server.
export function watchCrateSources(): AstroIntegration {
  return {
    name  : 'prose-watch-crate',
    hooks : {
      'astro:config:setup': ({ addWatchFile, config }) => {
        addWatchFile(ruleSourcePath(config.root))
        addWatchFile(cargoTomlPath(config.root))
        for (const binary of proseBinaryCandidates(config.root)) addWatchFile(binary)
      }
    }
  }
}
