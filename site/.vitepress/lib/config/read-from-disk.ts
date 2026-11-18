import fs   from 'node:fs'
import path from 'node:path'

import { normalizePath, type Plugin } from 'vite'

// Adds each file a module reads with `fs` to that module's watched files, which
// the `vitest --changed` walk follows as edges of the module graph, selecting
// the tests that import a reader when a file its globs match changes. `readers`
// keys each module by its path under `root` beside the globs it reads, relative
// to `root` as well, and a module or glob matching no file throws. The files a
// reader's globs match also go into that reader's `fsModuleCache` key, since
// Vitest otherwise serves the cached transform without calling `transform`.
export function readFromDiskPlugin(
  readers : Readonly<Record<string, readonly string[]>>,
  root    : string
): Plugin {
  const stale = Object.entries(readers)
    .flatMap(([module, globs]) => [module, ...globs])
    .filter(glob => fs.globSync(glob, { cwd: root }).length === 0)
  if (stale.length > 0) throw new Error(`nothing on disk matches ${stale.join(', ')}`)
  const globsOf = (id: string) => readers[normalizePath(path.relative(root, id))]
  return {
    name: 'prose-read-from-disk',
    configureVitest({ defineCacheKeyGenerator }) {
      defineCacheKeyGenerator(({ id }) => {
        const globs = globsOf(id)
        return globs && fs.globSync(globs, { cwd: root }).toSorted().join('\n')
      })
    },
    transform(_code, id) {
      const globs = globsOf(id)
      if (!globs) return
      for (const file of fs.globSync(globs, { cwd: root })) this.addWatchFile(path.join(root, file))
    }
  }
}
