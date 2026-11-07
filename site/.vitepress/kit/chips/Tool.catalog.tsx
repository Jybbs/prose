import type { Declaration } from '../catalog'
import { Tool }             from './Tool'

export const declaration: Declaration = {
  examples : [
    { label: 'Icon and name', render: () => <Tool slug="uv" /> }
  ],
  use      : 'Links to a tool by its registry slug, drawing its icon beside its name.'
}
