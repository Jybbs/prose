import path from 'node:path'

import type { Component } from 'vue'

import { subdirNames } from '../lib/fixtures/walker'
import { parseToml }   from '../lib/shared/toml'

const COMPONENT_MODULES = import.meta.glob<{ default: Component }>('../theme/components/**/*.vue')
const FIXTURES_ROOT     = path.join(import.meta.dirname, 'components')

export interface ComponentFixture {
  axeIgnore : readonly string[]
  component : string
  dir       : string
  id        : string
  props     : Record<string, unknown>
  slots     : Record<string, string>
}

interface FixtureMeta {
  axe       ?: { ignore?: readonly string[] }
  component  : string
  props     ?: Record<string, unknown>
  slots     ?: Record<string, string>
}

export function componentFixtures(): ComponentFixture[] {
  return subdirNames(FIXTURES_ROOT).flatMap(domain =>
    subdirNames(path.join(FIXTURES_ROOT, domain)).flatMap(component =>
      subdirNames(path.join(FIXTURES_ROOT, domain, component)).map(caseName => {
        const dir  = path.join(FIXTURES_ROOT, domain, component, caseName)
        const meta = parseToml(path.join(dir, 'meta.toml')) as unknown as FixtureMeta
        return {
          axeIgnore : meta.axe?.ignore ?? [],
          component : meta.component,
          dir       : dir,
          id        : `${domain}/${component}/${caseName}`,
          props     : meta.props ?? {},
          slots     : meta.slots ?? {}
        }
      })))
}

export async function loadComponent(name: string): Promise<Component> {
  const loader = COMPONENT_MODULES[`../theme/components/${name}.vue`]
  if (loader === undefined) {
    throw new Error(`fixture tree: no component at theme/components/${name}.vue`)
  }
  return (await loader()).default
}
