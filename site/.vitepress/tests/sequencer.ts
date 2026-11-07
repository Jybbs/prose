import { BaseSequencer }          from 'vitest/node'
import type { TestSpecification } from 'vitest/node'

const SWEEP = '/tests/pages/sweep.test.ts'

// Keeps the page sweep on every shard, which reads its own share of the built
// pages, and splits every other test file across the shards by a hash of its path.
// A sharded run holding a file of the `pages` project but no sweep fails.
export class SweepSequencer extends BaseSequencer {
  override async shard(files: TestSpecification[]): Promise<TestSpecification[]> {
    const { rest = [], sweep = [] } = Object.groupBy(files, file => file.moduleId.endsWith(SWEEP) ? 'sweep' : 'rest')
    if (sweep.length === 0 && rest.some(file => file.project.name === 'pages')) throw new Error(`no test file of the pages project ends with ${SWEEP}`)
    return [...sweep, ...await super.shard(rest)]
  }
}
