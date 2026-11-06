import path from 'node:path'

import { preview }          from 'vite'
import type { TestProject } from 'vitest/node'

declare module 'vitest' {
  export interface ProvidedContext {
    siteUrl: string
  }
}

// Serves the pressed bundle for the length of the run, on the first free port at
// or above Vite's default, and provides its address to the tests.
export default async function serve(project: TestProject): Promise<() => Promise<void>> {
  const server = await preview({
    appType    : 'mpa',
    configFile : false,
    logLevel   : 'silent',
    root       : path.join(project.config.root, '.vitepress')
  })
  project.provide('siteUrl', server.resolvedUrls!.local[0]!)
  return () => server.close()
}
