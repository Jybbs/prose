import fs   from 'node:fs'
import path from 'node:path'

import { parse } from 'smol-toml'

import { subdirectories } from '../../src/lib/content/discovery/page'

export interface Case {
  dir     : string
  domain  : string
  meta    : Record<string, unknown>
  name    : string
  subject : string
}

const root = path.join(import.meta.dirname, '..', 'fixtures')

export const cases = (): Case[] =>
  subdirectories(root).flatMap(domain =>
    subdirectories(path.join(root, domain)).flatMap(subject =>
      subdirectories(path.join(root, domain, subject)).map(name => {
        const dir  = path.join(root, domain, subject, name)
        const toml = path.join(dir, 'meta.toml')
        return {
          dir,
          domain,
          meta : fs.existsSync(toml) ? parse(fs.readFileSync(toml, 'utf8')) : {},
          name,
          subject
        }
      })))
