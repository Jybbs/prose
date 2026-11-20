interface DismissedHover {
  dismiss : () => void
  enter   : (element: HTMLElement | null) => boolean
  leave   : () => void
}

// Holds the element under the pointer and the one a dismissal left the pointer
// on. `enter` records the element the pointer enters and returns whether it is
// that dismissed one, which stays dismissed until `leave` runs or the pointer
// enters another element.
export function useDismissedHover(): DismissedHover {
  let dismissed : HTMLElement | null = null
  let pointed   : HTMLElement | null = null
  return {
    dismiss: () => { dismissed = pointed },

    enter: element => {
      pointed = element
      if (element !== null && element === dismissed) return true
      dismissed = null
      return false
    },

    leave: () => {
      dismissed = null
      pointed   = null
    }
  }
}
