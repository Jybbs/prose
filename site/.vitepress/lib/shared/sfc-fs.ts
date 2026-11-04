import { existsSync, readFileSync } from 'node:fs'

// Gives `@vue/compiler-sfc` the file system it resolves an imported prop type through. Its fallback
// reads `sys` from the `typescript` package, and TypeScript 7 exports no `sys`.
export const sfcFs = {
  fileExists : existsSync,
  readFile   : (file: string) => readFileSync(file, 'utf8')
}
