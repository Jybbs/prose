// Converts a fixture directory name to the kebab-case slug the registries key on.
export function ruleSlug(fixtureRule: string): string {
  return fixtureRule.replaceAll('_', '-')
}
