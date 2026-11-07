import { BaseSequencer }          from 'vitest/node'
import type { TestSpecification } from 'vitest/node'

const SWEEP = '/tests/pages/sweep.test.ts'

// Keeps the page sweep on every shard, which reads its own share of the built
// pages, and splits every other test file across the shards by a hash of its path.
export class SweepSequencer extends BaseSequencer {
  override async shard(files: TestSpecification[]): Promise<TestSpecification[]> {
    const { rest = [], sweep = [] } = Object.groupBy(files, file => file.moduleId.endsWith(SWEEP) ? 'sweep' : 'rest')
    return [...sweep, ...await super.shard(rest)]
  }
}
