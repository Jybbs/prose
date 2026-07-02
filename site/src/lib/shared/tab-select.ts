export interface TabSelectOptions {
  activeClass ?: string
  events       : readonly ('click' | 'focus' | 'mouseenter')[]
  key          : string
  panels       : string
  tabs         : string
}

// One listener set per tab, selection toggling the active class on the tabs,
// `aria-selected` where a tab carries the role, and `hidden` on the panels
// whose dataset `key` misses the selection.
export function wireTabSelect(root: ParentNode, options: TabSelectOptions): void {
  const { activeClass = 'is-active', events, key, panels, tabs } = options

  const select = (value: string): void => {
    for (const tab of root.querySelectorAll<HTMLElement>(tabs)) {
      const active = tab.dataset[key] === value
      tab.classList.toggle(activeClass, active)
      if (tab.getAttribute('role') === 'tab') tab.setAttribute('aria-selected', String(active))
    }
    for (const panel of root.querySelectorAll<HTMLElement>(panels)) {
      panel.hidden = panel.dataset[key] !== value
    }
  }

  for (const tab of root.querySelectorAll<HTMLElement>(tabs)) {
    for (const event of events) {
      tab.addEventListener(event, () => select(tab.dataset[key] ?? ''))
    }
  }
}
