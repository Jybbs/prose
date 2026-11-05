import type { Declaration } from '../catalog'
import { FamilyChip }       from './FamilyChip'

export const declaration: Declaration = {
  examples : [
    { label: 'Linked to its family', render: () => <FamilyChip family="alignment" /> },
    { label: 'Unlinked', render: () => <FamilyChip family="ordering" linked={false} /> }
  ],
  use      : 'Names a rule family beside its drawn nib, reading the family of the current rule where none is passed.'
}
