import vue              from '@vitejs/plugin-vue'
import { defineConfig } from 'vitest/config'

export default defineConfig({
  plugins: [vue()],
  test: {
    environment : 'node',
    globals     : true,
    root        : import.meta.dirname,
    include     : ['.vitepress/tests/**/*.test.ts'],
    coverage: {
      provider         : 'v8',
      reporter         : ['text', 'lcovonly'],
      reportsDirectory : 'coverage',
      include          : ['.vitepress/lib/**'],
      exclude: [
        '.vitepress/lib/og/render/**',
        '.vitepress/lib/rules/config-presets.ts',
        '.vitepress/lib/shared/fixture-tab.ts',
        '.vitepress/lib/shared/tools.ts'
      ],
      thresholds: {
        branches   : 90,
        functions  : 95,
        lines      : 95,
        statements : 95
      }
    }
  }
})
