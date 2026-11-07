import type { Declaration }       from '../catalog'
import { Popup }                  from './Popup'
import type { PopupTriggerScope } from './Popup'

export const declaration: Declaration = {
  examples : [
    {
      label  : 'Click the term or press Enter on it',
      render : () => (
        <Popup label="A term" placement="top" theme="glossary">
          {{
            default : () => <div>A note the term opens.</div>,
            trigger : ({ props }: PopupTriggerScope) => <button {...props} class="border-none p-0 font-mono text-sm text-brand-1" type="button">a term</button>
          }}
        </Popup>
      )
    }
  ],
  use      : 'Opens a card against a trigger on a click, a tap, or a key press and takes focus into it, or against an element the caller names where one listener serves many anchors, passively where the card follows a hover.'
}
