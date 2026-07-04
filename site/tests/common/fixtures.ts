import fs   from 'node:fs'
import path from 'node:path'

import { parse } from 'smol-toml'

export interface Case {
  dir     : string
  domain  : string
  meta    : Record<string, unknown>
  name    : string
  subject : string
}

const root = path.join(import.meta.dirname, '..', 'fixtures')

const subdirs = (at: string): string[] =>
  fs.readdirSync(at, { withFileTypes: true }).filter(entry => entry.isDirectory()).map(entry => entry.name).sort()

export const cases = (): Case[] =>
  subdirs(root).flatMap(domain =>
    subdirs(path.join(root, domain)).flatMap(subject =>
      subdirs(path.join(root, domain, subject)).map(name => {
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
