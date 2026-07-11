import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    environment : 'node',
    globals     : true,
    include     : ['.vitepress/tests/wasm/**/*.test.ts'],
    reporters   : process.env.GITHUB_ACTIONS ? ['default', 'github-actions'] : ['default'],
    root        : import.meta.dirname
  }
})
