import type { Declaration } from '../catalog'
import { Chip }             from './Chip'

export const declaration: Declaration = {
  examples : [
    {
      label  : 'A rule, linked',
      render : () => <Chip href="/rules/alignment/align-equals" variant="rule-chip">align-equals</Chip>
    },
    {
      label  : 'A family, unlinked',
      render : () => <Chip family="alignment" variant="family-chip">Alignment</Chip>
    }
  ],
  use      : 'Sets a rule or a family in a small mono label, which becomes a link where it takes an address.'
}
