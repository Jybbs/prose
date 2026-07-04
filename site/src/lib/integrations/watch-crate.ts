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

if (import.meta.vitest) {
  const { describe, expect, test, vi } = import.meta.vitest

  type SetupHook    = NonNullable<AstroIntegration['hooks']['astro:config:setup']>
  type SetupContext = Parameters<SetupHook>[0]

  describe('watchCrateSources', () => {
    test('registers a named integration', () => {
      expect(watchCrateSources().name).toBe('prose-watch-crate')
    })

    test('watches the rule source, the manifest, and both binary candidates', () => {
      const addWatchFile = vi.fn()
      const setup        = watchCrateSources().hooks['astro:config:setup']
      expect(setup).toBeDefined()
      setup!({ addWatchFile, config: { root: new URL('file:///repo/site/') } } as unknown as SetupContext)

      const watched = addWatchFile.mock.calls.map(([path]) => String(path))
      expect(watched.some(path => path.endsWith('src/rule.rs'))).toBe(true)
      expect(watched.some(path => path.endsWith('Cargo.toml'))).toBe(true)
      expect(watched.filter(path => path.endsWith('/prose'))).toHaveLength(2)
    })
  })
}
