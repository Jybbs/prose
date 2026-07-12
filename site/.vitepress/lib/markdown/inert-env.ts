// The env contract between the loader render wrappers and the markdown
// plugins. An inert render emits plain elements that survive `v-html`
// injection in place of Vue component tags, which only compiled page
// bodies can mount.
export interface InertEnv {
  inertHtml?  : boolean
  plainTerms? : boolean
}

export function inertEnv(): InertEnv {
  return { inertHtml: true }
}

export function isInert(env: InertEnv): boolean {
  return env.inertHtml === true
}

// Caption renders live inside cover-linked cards and hover poppers, where a
// glossary anchor cannot receive its own click, so the plain-terms env drops
// the anchor and keeps the text.
export function plainTermsEnv(): InertEnv {
  return { inertHtml: true, plainTerms: true }
}

export function isPlainTerms(env: InertEnv): boolean {
  return env.plainTerms === true
}
