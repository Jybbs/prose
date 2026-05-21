import { watchEffect } from 'vue'

import { useCurrentFamily } from './route'

export function useFamilyBodyAttr(): void {
  const family = useCurrentFamily()
  watchEffect(() => {
    if (typeof document === 'undefined') return
    if (family.value) document.body.setAttribute('data-family', family.value)
    else              document.body.removeAttribute('data-family')
  })
}
