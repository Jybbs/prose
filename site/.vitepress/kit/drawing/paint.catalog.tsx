import type { Declaration } from '../catalog'
import { millis }           from './paint'

export const declaration: Declaration = {
  examples : [
    { label: 'A duration read in milliseconds', render: () => <code>{millis('0.2s')}</code> }
  ],
  use      : 'Reads the timing a stylesheet declares and waits on the next paint, for a script that moves in step with a transition.'
}
