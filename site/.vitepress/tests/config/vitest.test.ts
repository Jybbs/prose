import fs   from 'node:fs'
import path from 'node:path'

import pm from 'picomatch'

import config       from '../../../vitest.config'
import { repoRoot } from '../../lib/shared/paths'

const root          = repoRoot(import.meta.url)
const runsAllOfDocs = pm(config.test?.forceRerunTriggers ?? [], { dot: true })

describe('the force-rerun triggers', () => {
  it.each([
    '.github/scripts/platforms.toml',
    'Cargo.lock',
    'Cargo.toml',
    'crate/Cargo.toml',
    'crate/pyproject.toml',
    'crate/src/lib.rs',
    'crate/tests/fixtures/align_colons/aligned_dict_collapses_post_colon/input.py',
    'site/.vitepress/assets/mise-logo.svg',
    'site/.vitepress/tests/rules/fixtures/composition/alpha_case/config.toml',
    'site/.vitepress/theme/styles/tokens.css',
    'site/bun.lock',
    'site/index.md',
    'site/patches/magic-move-frame-cost.patch',
    'site/public/badges/coverage.svg'
  ])('match %s, which the docs project reads from disk', file => {
    expect(fs.existsSync(path.join(root, file))).toBe(true)
    expect(runsAllOfDocs(path.join(root, file))).toBe(true)
  })
})
