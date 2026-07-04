/// <reference types="vitest/config" />
import { getViteConfig } from 'astro/config'

export default getViteConfig({
  test: {
    environment   : 'node',
    globals       : true,
    include       : ['tests/**/*.test.ts'],
    includeSource : ['src/lib/**/*.ts'],
    reporters     : process.env.GITHUB_ACTIONS ? ['default', 'github-actions'] : ['default'],
    setupFiles    : ['./tests/common/setup.ts'],

    coverage: {
      exclude          : ['src/lib/og/assets.ts', 'src/lib/og/cache.ts', 'src/lib/og/render.ts'],
      include          : ['src/lib/**'],
      provider         : 'istanbul',
      reporter         : ['text', 'lcovonly'],
      reportsDirectory : 'coverage',
      thresholds       : { branches: 90, functions: 95, lines: 95, statements: 95 }
    }
  }
})
