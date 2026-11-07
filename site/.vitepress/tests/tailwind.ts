import fs                from 'node:fs'
import { createRequire } from 'node:module'
import path              from 'node:path'

import { Scanner }                      from '@tailwindcss/oxide'
import { __unstable__loadDesignSystem } from 'tailwindcss'

import { themeCss } from '../lib/shared/palette'

import { read, SITE } from './vocabulary'

import type { Root } from 'postcss'

const REQUIRE = createRequire(import.meta.url)

const ENTRY = '.vitepress/theme/styles/tailwind.css'

type DesignSystem = Awaited<ReturnType<typeof __unstable__loadDesignSystem>>

// Names each candidate an `@source not inline()` line of the Tailwind entry
// blocks.
export function blockedCandidates(): Set<string> {
  return new Set([...read(ENTRY).matchAll(/@source not inline\("([^"]+)"\)/g)]
    .flatMap(([, names]) => names.split(/\s+/)))
}

// Loads the design system Tailwind builds from its entry and the `@theme` block
// the build gives that entry, resolving each sheet the entry imports against the
// site or `node_modules`.
export function designSystem(): Promise<DesignSystem> {
  return __unstable__loadDesignSystem(read(ENTRY) + themeCss(), {
    base           : path.join(SITE, path.dirname(ENTRY)),
    loadStylesheet : async (id, base) => {
      const file = id.startsWith('.') ? path.resolve(base, id) : REQUIRE.resolve(id)
      return { base: path.dirname(file), content: await fs.promises.readFile(file, 'utf8'), path: file }
    }
  })
}

// Returns the CSS Tailwind compiles for each candidate its scanner finds in the
// entry's `@source` files and for each class an `@apply` in the `roots` names,
// leaving out the candidates the entry blocks.
export function generatedCss(design: DesignSystem, roots: Root[]): string {
  const applied = [] as string[]
  for (const root of roots) {
    root.walkAtRules('apply', rule => { applied.push(...rule.params.split(/\s+/)) })
  }
  const blocked = blockedCandidates()
  return design.candidatesToCss([...scannedCandidates(), ...applied].filter(name => !blocked.has(name)))
    .filter(Boolean)
    .join('\n')
}

// Lists each candidate Tailwind's scanner finds in the files the `@source`
// lines of its entry name.
export function scannedCandidates(): string[] {
  const base = path.join(SITE, path.dirname(ENTRY))
  return new Scanner({ sources: sourceGlobs().map(pattern => ({ base, negated: false, pattern })) }).scan()
}

// Lists each file an `@source` line of the Tailwind entry names, relative to
// the site.
export function sourceFiles(): string[] {
  return sourceGlobs()
    .flatMap(glob => fs.globSync(path.join(path.dirname(ENTRY), glob), { cwd: SITE }))
    .toSorted()
}

// Names each glob an `@source` line of the Tailwind entry writes.
function sourceGlobs(): string[] {
  return [...read(ENTRY).matchAll(/@source "([^"]+)"/g)].map(([, glob]) => glob)
}
