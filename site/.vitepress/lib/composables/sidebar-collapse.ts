import { nextTick, onBeforeUnmount, onMounted, watch } from 'vue'
import { useRoute }                                    from 'vitepress'

const STORAGE_KEY = 'prose:sidebar:collapsed'

function loadState(): Record<string, boolean> {
  if (typeof sessionStorage === 'undefined') return {}
  try {
    const raw = sessionStorage.getItem(STORAGE_KEY)
    return raw ? JSON.parse(raw) as Record<string, boolean> : {}
  } catch {
    return {}
  }
}

function saveState(state: Record<string, boolean>): void {
  if (typeof sessionStorage === 'undefined') return
  try {
    sessionStorage.setItem(STORAGE_KEY, JSON.stringify(state))
  } catch {
    // sessionStorage quota or unavailable: collapsed state will not persist, which is acceptable.
  }
}

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
  root.querySelectorAll<HTMLElement>('.VPSidebarItem.level-0.collapsible').forEach(fn)
}

function restoreAll(root: ParentNode, state: Record<string, boolean>): void {
  eachCollapsibleGroup(root, group => {
    if (isActiveGroup(group)) {
      syncGroup(group, false)
      return
    }
    if (state[keyFor(group) ?? ''] === true) syncGroup(group, true)
  })
}

function persistFromDom(root: ParentNode): void {
  const prior = loadState()
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
  saveState(next)
}

export function useSidebarCollapse(): void {
  if (typeof window === 'undefined') return

  const route      = useRoute()
  let   attachedTo : HTMLElement | null = null

  const onSidebarClick   = (): void => { void nextTick(() => attachedTo && persistFromDom(attachedTo)) }
  const onSidebarKeydown = (event: KeyboardEvent): void => {
    if (event.key !== ' ' && event.key !== 'Spacebar') return
    const target = event.target as HTMLElement | null
    const caret  = target?.closest<HTMLElement>('.VPSidebarItem.collapsible > .item > .caret')
    if (!caret) return
    event.preventDefault()
    caret.click()
  }

  const wire = (): void => {
    const sidebar = document.querySelector<HTMLElement>('.VPSidebar')
    if (!sidebar || sidebar === attachedTo) {
      if (sidebar) restoreAll(sidebar, loadState())
      return
    }
    if (attachedTo) {
      attachedTo.removeEventListener('click',   onSidebarClick)
      attachedTo.removeEventListener('keydown', onSidebarKeydown, true)
    }
    sidebar.addEventListener('click',   onSidebarClick)
    sidebar.addEventListener('keydown', onSidebarKeydown, true)
    attachedTo = sidebar
    restoreAll(sidebar, loadState())
  }

  onMounted(() => { void nextTick(wire) })

  watch(() => route.path, () => { void nextTick(wire) })

  onBeforeUnmount(() => {
    if (attachedTo) {
      attachedTo.removeEventListener('click',   onSidebarClick)
      attachedTo.removeEventListener('keydown', onSidebarKeydown, true)
      attachedTo = null
    }
  })
}
