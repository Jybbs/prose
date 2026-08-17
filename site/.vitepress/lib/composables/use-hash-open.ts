import { useEventListener } from '@vueuse/core'
import { onMounted }        from 'vue'

// Reports the bare fragment on mount and on every later hash change.
export function useHashOpen(onFragment: (fragment: string) => void): void {
  const report = (): void => onFragment(window.location.hash.slice(1))
  onMounted(report)
  useEventListener('hashchange', report)
}
