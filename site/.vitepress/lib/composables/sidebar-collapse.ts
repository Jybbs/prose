import { useEventListener, useSessionStorage } from '@vueuse/core'
import { nextTick, onMounted, ref, watch, type Ref } from 'vue'
import { useRoute } from 'vitepress'

const STORAGE_KEY = 'prose:sidebar:collapsed'

function keyFor(item: HTMLElement): string | null {
  const text = item.querySelector<HTMLElement>(':scope > .item .text')?.textContent?.trim()
  return text ?? null
}

function isActiveGroup(item: HTMLElement): boolean {
  return item.classList.contains('is-active') || item.classList.contains('has-active')
}

function clickCaret(item: HTMLElement): void {
  item.querySelector<HTMLElement>(':scope > .item > .caret')?.click()
}

function syncGroup(item: HTMLElement, target: boolean): void {
  const current = item.classList.contains('collapsed')
  if (current !== target) clickCaret(item)
}

function eachCollapsibleGroup(root: ParentNode, fn: (group: HTMLElement) => void): void {
  Iterator.from(root.querySelectorAll<HTMLElement>('.VPSidebarItem.level-0.collapsible')).forEach(fn)
}

function restoreAll(root: ParentNode, state: Record<string, boolean>): void {
  Iterator.from(root.querySelectorAll<HTMLElement>('.VPSidebarItem.level-0.collapsible'))
    .forEach(g => {
      if (isActiveGroup(g))                      syncGroup(g, false)
      else if (state[keyFor(g) ?? ''] === true)  syncGroup(g, true)
    })
}

function persistFromDom(root: ParentNode, state: Ref<Record<string, boolean>>): void {
  const prior = state.value
  const next  : Record<string, boolean> = {}
  eachCollapsibleGroup(root, group => {
    const key = keyFor(group)
    if (!key) return
    if (isActiveGroup(group)) {
      if (prior[key] === true) next[key] = true
      return
    }
    if (group.classList.contains('collapsed')) next[key] = true
  })
  state.value = next
}

export function useSidebarCollapse(): void {
  if (typeof window === 'undefined') return

  const state      = useSessionStorage<Record<string, boolean>>(STORAGE_KEY, {})
  const route      = useRoute()
  const sidebarRef = ref<HTMLElement | null>(null)

  useEventListener(sidebarRef, 'click',   () => {
    void nextTick(() => sidebarRef.value && persistFromDom(sidebarRef.value, state))
  })
  useEventListener(sidebarRef, 'keydown', (event: KeyboardEvent) => {
    if (event.key !== ' ' && event.key !== 'Spacebar') return
    const target = event.target as HTMLElement | null
    const caret  = target?.closest<HTMLElement>('.VPSidebarItem.collapsible > .item > .caret')
    if (!caret) return
    event.preventDefault()
    caret.click()
  }, true)

  const wire = (): void => {
    sidebarRef.value = document.querySelector<HTMLElement>('.VPSidebar')
    if (sidebarRef.value) restoreAll(sidebarRef.value, state.value)
  }

  onMounted(() => { void nextTick(wire) })
  watch(() => route.path, () => { void nextTick(wire) })
}
