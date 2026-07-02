import astro from 'eslint-plugin-astro'

// The `.astro` frontmatter and template surface oxlint cannot parse. The
// TypeScript tree is oxlint's, configured in `.oxlintrc.json`.
export default [
  ...astro.configs['flat/recommended'],
  { ignores: ['.astro/', 'dist/'] }
]
