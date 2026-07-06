// The env contract between the loader render wrappers and the markdown
// plugins. An inert render emits plain elements that survive `v-html`
// injection in place of Vue component tags, which only compiled page
// bodies can mount.
export interface InertEnv {
  inertHtml?: boolean
}

export function inertEnv(): InertEnv {
  return { inertHtml: true }
}

export function isInert(env: InertEnv): boolean {
  return env.inertHtml === true
}
