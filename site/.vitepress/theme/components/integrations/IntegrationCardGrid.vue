<script setup lang="ts">
interface Card {
  body    : string
  href    : string
  kicker  : string
  title   : string
}

const CARDS: readonly Card[] = [
  {
    body   : 'Run-on-save against the editor’s save hook, or structured diagnostics surfaced inline in the LSP panel as you type.',
    href   : '/integrations/editor',
    kicker : 'save · LSP',
    title  : 'Editor'
  },
  {
    body   : 'Refuses a commit that would land unformatted code, gating the git staging boundary against layout drift.',
    href   : '/integrations/pre-commit',
    kicker : 'git staging',
    title  : 'Pre-Commit'
  },
  {
    body   : 'Fails a PR check on a layout regression, with workflow-command annotations inline and SARIF upload to the Security tab.',
    href   : '/integrations/github-actions',
    kicker : 'CI runner · code scanning',
    title  : 'GitHub Actions'
  },
  {
    body   : 'Tab-completes every flag and rule slug across Bash, Zsh, Fish, Elvish, Nushell, and PowerShell.',
    href   : '/integrations/shell-completions',
    kicker : 'interactive shell',
    title  : 'Shell Completions'
  },
  {
    body   : 'For projects that already run Ruff, the <code>extend-ignore</code> configuration that keeps Ruff’s <code>pycodestyle</code> codes from flagging the whitespace <em>Prose</em>’s alignment rules deliberately introduce.',
    href   : '/integrations/ruff',
    kicker : 'alongside Ruff',
    title  : 'Ruff'
  }
]
</script>

<template>
  <div class="int-card-grid">
    <a v-for="card in CARDS" :key="card.href" class="int-card" :href="card.href">
      <p class="int-kicker">{{ card.kicker }}</p>
      <p class="int-title">{{ card.title }}</p>
      <p class="int-body" v-html="card.body"></p>
      <p class="int-foot">Open →</p>
    </a>
  </div>
</template>
