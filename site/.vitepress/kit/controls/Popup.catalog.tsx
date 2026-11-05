import type { Declaration }       from '../catalog'
import { Popup }                  from './Popup'
import type { PopupTriggerScope } from './Popup'

export const declaration: Declaration = {
  examples : [
    {
      label  : 'Hover or focus the term',
      render : () => (
        <Popup placement="top" theme="glossary">
          {{
            default : () => <div>A note the term opens.</div>,
            trigger : ({ props }: PopupTriggerScope) => <button {...props} class="border-none p-0 font-mono text-sm text-brand-1" type="button">a term</button>
          }}
        </Popup>
      )
    }
  ],
  use      : 'Opens a note against a trigger on hover or focus, or against an element the caller names where one listener serves many anchors.'
}
