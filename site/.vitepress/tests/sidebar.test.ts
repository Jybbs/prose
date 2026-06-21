import type { DiscoveredPrimitive } from '../lib/primitives/discovery'
import type { DiscoveredRule }      from '../lib/rules/discovery'
import { buildSidebar }             from '../lib/config/sidebar'

const rules: readonly DiscoveredRule[] = [
  {
    caption  : 'Align consecutive assignments',
    category : 'auto-fix',
    family   : 'alignment',
    href     : '/rules/alignment/align-equals',
    related  : [],
    slug     : 'align-equals'
  },
  {
    caption  : 'Alphabetize sibling entries',
    category : 'auto-fix',
    family   : 'ordering',
    href     : '/rules/ordering/alphabetize',
    related  : [],
    slug     : 'alphabetize'
  }
]

const primitives: readonly Pick<DiscoveredPrimitive, 'name' | 'slug' | 'stability'>[] = [
  { name: 'Aligner',  slug: 'aligner',  stability: 'public'   },
  { name: 'Pipeline', slug: 'pipeline', stability: 'internal' }
]

describe('buildSidebar', () => {
  it('builds the route-keyed sidebar tree', () => {
    expect(buildSidebar(rules, primitives)).toMatchSnapshot()
  })
})
