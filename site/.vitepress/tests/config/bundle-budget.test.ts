import fs   from 'node:fs'
import path from 'node:path'

import { checkBundleBudget, measureBundle } from '../../lib/config/bundle-budget'
import { supportTest }                      from '../support'

const write = (dir: string, file: string, bytes: number) => {
  fs.mkdirSync(path.dirname(path.join(dir, file)), { recursive: true })
  fs.writeFileSync(path.join(dir, file), 'x'.repeat(bytes))
}

describe('measureBundle', () => {
  supportTest('sums the CSS and the JavaScript at every depth, leaving other files out', ({ tmpDir }) => {
    write(tmpDir, 'assets/style.css', 30)
    write(tmpDir, 'assets/vp-icons.css', 5)
    write(tmpDir, 'assets/app.js', 100)
    write(tmpDir, 'assets/chunks/theme.js', 20)
    write(tmpDir, 'index.html', 7)
    write(tmpDir, 'hashmap.json', 9)
    expect(measureBundle(tmpDir)).toStrictEqual({ css: 35, js: 120 })
  })
})

describe('checkBundleBudget', () => {
  supportTest('passes a bundle that measures exactly its budget', ({ tmpDir }) => {
    write(tmpDir, 'a.css', 10)
    write(tmpDir, 'a.js', 20)
    expect(() => checkBundleBudget(tmpDir, { css: 10, js: 20 })).not.toThrow()
  })

  supportTest.for([
    {
      budget  : { css: 9, js: 20 },
      message : "the built site's css measures 10 bytes against a budget of 9"
    },
    {
      budget  : { css: 10, js: 19 },
      message : "the built site's js measures 20 bytes against a budget of 19"
    },
    {
      budget  : { css: 9, js: 19 },
      message : "the built site's css measures 10 bytes against a budget of 9 and the built site's js "
              + 'measures 20 bytes against a budget of 19'
    }
  ])('names each kind a bundle grows past, against $budget', ({ budget, message }, { tmpDir }) => {
    write(tmpDir, 'a.css', 10)
    write(tmpDir, 'a.js', 20)
    expect(() => checkBundleBudget(tmpDir, budget)).toThrow(message)
  })
})
