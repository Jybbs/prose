interface DismissedHover {
  dismiss : () => void
  enter   : (element: HTMLElement | null) => boolean
  leave   : () => void
  move    : (element: HTMLElement | null) => void
}

// Holds the element under the pointer and the one a dismissal left the pointer
// on. `enter` records the element a `mouseover` lands on and returns whether it
// is that dismissed one, which stays dismissed until a `pointermove` passed to
// `move` lands outside it or `leave` runs, so a `mouseover` with no motion, such
// as one a page sends as its content shifts under a still pointer, clears nothing.
export function useDismissedHover(): DismissedHover {
  let dismissed : HTMLElement | null = null
  let pointed   : HTMLElement | null = null
  return {
    dismiss : () => { dismissed = pointed },
    move    : element => { if (element !== dismissed) dismissed = null },

    enter: element => {
      pointed = element
      return element !== null && element === dismissed
    },

    leave: () => {
      dismissed = null
      pointed   = null
    }
  }
}
