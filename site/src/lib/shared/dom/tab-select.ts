interface TabSelectOptions {
  activeClass ?: string
  events       : readonly ('click' | 'focus' | 'mouseenter')[]
  key          : string
  panels       : string
  tabs         : string
}

// One listener set per tab, selection toggling the active class onto the
// picked tab, `aria-selected` where a tab carries the role, and `hidden` on
// the panels whose dataset `key` misses the selection, a tab carrying no key
// value showing every panel.
function wireTabSelect(options: TabSelectOptions, root: ParentNode): void {
  const { activeClass = 'is-active', events, key, panels, tabs } = options

  const select = (picked: HTMLElement): void => {
    const value = picked.dataset[key]
    for (const tab of root.querySelectorAll<HTMLElement>(tabs)) {
      const active = tab === picked
      tab.classList.toggle(activeClass, active)
      if (tab.getAttribute('role') === 'tab') tab.setAttribute('aria-selected', String(active))
    }
    for (const panel of root.querySelectorAll<HTMLElement>(panels)) {
      panel.hidden = value !== undefined && panel.dataset[key] !== value
    }
  }

  for (const tab of root.querySelectorAll<HTMLElement>(tabs)) {
    for (const event of events) {
      tab.addEventListener(event, () => select(tab))
    }
  }
}

// Registers a custom element whose whole behavior is one `wireTabSelect`
// call over its subtree.
export function defineTabSelectElement(options: TabSelectOptions, tag: string): void {
  customElements.define(tag, class extends HTMLElement {
    connectedCallback(): void {
      wireTabSelect(options, this)
    }
  })
}
