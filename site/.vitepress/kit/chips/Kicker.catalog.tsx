import type { Declaration } from '../catalog'
import { Kicker }           from './Kicker'

export const declaration: Declaration = {
  examples : [
    { label: 'Plain', render: () => <Kicker>{{ default: () => 'Read on' }}</Kicker> },
    { label: 'Under a rule', render: () => <Kicker variant="rule">{{ default: () => 'Read on' }}</Kicker> },
    { label: 'Widely spaced', render: () => <Kicker variant="wide">{{ default: () => 'Read on' }}</Kicker> }
  ],
  use      : 'Sets the small uppercase line that opens a section.'
}
