/** @type {import('@stryker-mutator/api/core').PartialStrykerOptions} */
export default {
  checkers         : ['typescript'],
  coverageAnalysis : 'perTest',
  incremental      : true,
  testRunner       : 'vitest',
  thresholds       : { break: 70, high: 90, low: 75 },
  tsconfigFile     : 'tsconfig.json',
  mutate           : [
    'src/lib/**/*.ts',
    '!src/lib/og/assets.ts',
    '!src/lib/og/cache.ts',
    '!src/lib/og/render.ts'
  ]
}
