export const SHIKI_THEMES = { dark: 'github-dark', light: 'github-light' } as const

// The Python target versions the rule pages tab across, oldest first.
export const TARGET_VERSIONS = ['3.10', '3.11', '3.12', '3.13', '3.14'] as const

// The Starlight sync key that keeps every target-version tab group on a page in
// step and persists the choice across navigation.
export const TARGET_VERSION_SYNC_KEY = 'prose-target-version'
