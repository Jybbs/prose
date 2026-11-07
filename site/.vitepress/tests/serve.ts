import path from 'node:path'

import { preview }          from 'vite'
import type { TestProject } from 'vitest/node'

declare module 'vitest' {
  export interface ProvidedContext {
    shard   : { count: number, index: number }
    siteUrl : string
  }
}

// Serves the pressed bundle for the length of the run, on the first free port at
// or above Vite's default, and provides its address and the run's shard to the tests.
export default async function serve(project: TestProject): Promise<() => Promise<void>> {
  const server = await preview({
    appType    : 'mpa',
    configFile : false,
    logLevel   : 'silent',
    root       : path.join(project.config.root, '.vitepress')
  })
  project.provide('shard', project.globalConfig.shard ?? { count: 1, index: 1 })
  project.provide('siteUrl', server.resolvedUrls!.local[0]!)
  return () => server.close()
}
