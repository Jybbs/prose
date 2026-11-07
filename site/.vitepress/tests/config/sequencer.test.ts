import type { TestSpecification, Vitest } from 'vitest/node'

import { SweepSequencer } from '../sequencer'

const ROOT  = '/site'
const SWEEP = { moduleId: `${ROOT}/.vitepress/tests/pages/sweep.test.ts`, project: { name: 'pages' } } as TestSpecification
const REST  = ['a', 'b', 'c'].map(name => ({ moduleId: `${ROOT}/.vitepress/tests/${name}.test.ts`, project: { name: 'docs' } }) as TestSpecification)

const shard = (index: number, files: TestSpecification[]): Promise<TestSpecification[]> =>
  new SweepSequencer({ config: { root: ROOT, shard: { count: 4, index } } } as Vitest).shard(files)

describe('SweepSequencer', () => {
  it('keeps the page sweep on every shard and gives every other file to one shard', async () => {
    const shards = await Promise.all([1, 2, 3, 4].map(index => shard(index, [...REST, SWEEP])))
    expect(shards.every(files => files.includes(SWEEP))).toBe(true)
    expect(shards.flat().filter(file => file !== SWEEP).map(file => file.moduleId).toSorted())
      .toStrictEqual(REST.map(file => file.moduleId))
  })

  it('fails a sharded run holding a file of the pages project but no sweep', async () => {
    const stray = { moduleId: `${ROOT}/.vitepress/tests/pages/stray.test.ts`, project: { name: 'pages' } } as TestSpecification
    await expect(shard(1, [...REST, stray])).rejects.toThrow('no test file of the pages project')
  })
})
