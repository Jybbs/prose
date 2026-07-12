<script setup lang="ts">
import { refAutoReset, useClipboard, useStorage } from '@vueuse/core'
import { computed, onMounted, ref }               from 'vue'

import { useProseSandbox } from '../../../lib/composables/use-prose-sandbox'
import { data as schema }  from '../../../lib/sandbox/config-schema.data'
import { data as pool }    from '../../../lib/sandbox/pool.data'

const deckLocked     = useStorage('prose-sandbox-pinned', false)
const deckOpen       = useStorage('prose-sandbox-deck-open', true)
const draggingLength = ref('')
const guideHue       = ref('')
const previewLength  = ref(0)
const refreshArmed   = refAutoReset(false, 4000)
const sandbox        = useProseSandbox({ cases: pool.cases, schema })

function onDragging(key: string, hue: string): void {
  draggingLength.value = key
  guideHue.value       = hue
  if (key) previewLength.value = sandbox.lengthValue(key)
}

function onPreview(_key: string, value: number): void {
  previewLength.value = value
}

const { copied, copy } = useClipboard()
const { copied: linkCopied, copy: copyLink } = useClipboard()

async function shareLink(): Promise<void> {
  const url = await sandbox.share()
  if (url) copyLink(url)
}

// An edited source or a moved config is work worth guarding, so a dirty
// refresh arms first and discards only on the confirming second click. A
// source matching any pool example verbatim is nobody's work, whichever
// session it was restored from, so it refreshes without the arm.
const dirty = computed(() =>
  sandbox.configToml.value !== '' ||
  !pool.cases.some(entry => entry.source === sandbox.source.value))

const announcement = computed(() => {
  const findings = sandbox.diagnostics.value.length
  const lines    = sandbox.formatted.value.trimEnd().split('\n').length
  return `Formatted ${lines} ${lines === 1 ? 'line' : 'lines'}, `
       + `${findings} lint ${findings === 1 ? 'finding' : 'findings'}.`
})

function refresh(): void {
  if (refreshArmed.value || !dirty.value) {
    refreshArmed.value = false
    sandbox.refresh()
    return
  }
  refreshArmed.value = true
}

// The wasm glue imports client-side only, so the first random case formats
// on mount rather than on the static-generation path.
onMounted(sandbox.start)
</script>

<template>
  <div class="sandbox">
    <section class="sandbox-deck panel" :data-locked="deckLocked">
      <button
        type="button"
        class="sandbox-deck-head"
        :aria-expanded="deckOpen"
        :title="deckOpen ? 'Hide configuration' : 'Show configuration'"
        @click="deckOpen = !deckOpen"
      >
        <span class="kicker">Configuration</span>
      </button>
      <button
        type="button"
        class="sandbox-deck-lock"
        :aria-pressed="deckLocked"
        :title="deckLocked ? 'Unpin from scroll' : 'Pin while scrolling'"
        :aria-label="deckLocked ? 'Unpin the configuration' : 'Pin the configuration while scrolling'"
        @click.stop="deckLocked = !deckLocked"
      >
        <svg class="sandbox-deck-lock-icon glyph" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <rect x="5" y="11" width="14" height="9" rx="2" />
          <path :d="deckLocked ? 'M8 11V7a4 4 0 0 1 8 0v4' : 'M8 11V7a4 4 0 0 1 8 0'" />
        </svg>
      </button>
      <div v-show="deckOpen" class="sandbox-deck-body">
        <ProseSandboxControls
          :sandbox="sandbox"
          @dragging="onDragging"
          @preview="onPreview"
        />
      </div>
    </section>

    <div class="sandbox-surfaces">
      <div class="sandbox-py copy-host">
        <ProseSandboxSurface
          :sandbox="sandbox"
          :guide="draggingLength ? previewLength : null"
          :guide-hue="guideHue"
        />
        <button
          type="button"
          class="copy"
          :class="{ copied }"
          :title="copied ? 'Copied' : 'Copy the formatted Python'"
          :aria-label="copied ? 'Copied' : 'Copy the formatted Python'"
          @click="copy(sandbox.formatted.value)"
        />
        <button
          type="button"
          class="sandbox-corner sandbox-refresh"
          :data-armed="refreshArmed || null"
          :title="refreshArmed ? 'Click again to proceed with a new example' : 'New example'"
          :aria-label="refreshArmed ? 'Click again to proceed with a new example' : 'New example'"
          @click="refresh"
        >
          <svg class="glyph" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8" />
            <path d="M21 3v5h-5" />
            <path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16" />
            <path d="M8 16H3v5" />
          </svg>
        </button>
        <button
          type="button"
          class="sandbox-corner sandbox-share"
          :title="linkCopied ? 'Link copied' : 'Copy a link to this sandbox'"
          :aria-label="linkCopied ? 'Link copied' : 'Copy a link to this sandbox'"
          @click="shareLink"
        >
          <svg class="glyph" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path v-if="linkCopied" d="M4 12.5l5 5L20 6.5" />
            <template v-else>
              <path d="M10 13a5 5 0 0 0 7.5.5l3-3a5 5 0 0 0-7-7l-1.7 1.7" />
              <path d="M14 11a5 5 0 0 0-7.5-.5l-3 3a5 5 0 0 0 7 7l1.7-1.7" />
            </template>
          </svg>
        </button>
        <span v-if="refreshArmed" class="sandbox-refresh-tip" role="status">
          Click again to proceed with a new example
        </span>
        <p class="sandbox-announce" aria-live="polite">{{ announcement }}</p>
      </div>
      <ProseSandboxToml :sandbox="sandbox" />
    </div>
  </div>
</template>

<style scoped>
.sandbox {
  position  : relative;
  left      : 50%;
  width     : min(94rem, calc(100vw - 3rem));
  transform : translateX(-50%);
}

.sandbox-surfaces {
  display               : grid;
  grid-template-columns : minmax(0, 2fr) minmax(0, 1fr);
  gap                   : 1.25rem;
  align-items           : stretch;
}

.sandbox-py {
  position  : relative;
  display   : grid;
  min-width : 0;
}

.sandbox-corner {
  position      : absolute;
  top           : 8px;
  z-index       : 3;
  display       : grid;
  place-items   : center;
  width         : 28px;
  height        : 28px;
  padding       : 0;
  border        : 1px solid var(--vp-code-copy-code-border-color);
  border-radius : var(--prose-radius-sm);
  background    : var(--vp-code-copy-code-bg);
  color         : var(--vp-c-text-2);
  cursor        : pointer;
  opacity       : 0;
  transition    : opacity var(--prose-transition-slow), color var(--prose-transition);
}

.sandbox-refresh {
  right : 40px;
}

.sandbox-share {
  right : 72px;
}

.sandbox-py:hover > .sandbox-corner,
.sandbox-corner:focus-visible,
.sandbox-refresh[data-armed] {
  opacity : 1;
}

.sandbox-refresh-tip {
  position       : absolute;
  top            : 8px;
  right          : 108px;
  z-index        : 3;
  display        : flex;
  align-items    : center;
  height         : 28px;
  padding        : 0 10px;
  border         : 1px solid color-mix(in srgb, var(--prose-role-warning) 55%, var(--vp-c-divider));
  border-radius  : var(--prose-radius-sm);
  background     : var(--vp-c-bg);
  box-shadow     : var(--prose-shadow-tooltip);
  color          : var(--vp-c-text-1);
  font-family    : var(--vp-font-family-mono);
  font-size      : var(--prose-text-xxs);
  letter-spacing : 0.04em;
  white-space    : nowrap;
  pointer-events : none;
}

.sandbox-corner:hover {
  color : var(--vp-c-text-1);
}

.sandbox-refresh[data-armed] {
  border-color : var(--prose-role-warning);
  color        : var(--prose-role-warning);
}

.sandbox-corner:focus-visible {
  outline        : var(--prose-focus-ring);
  outline-offset : 1px;
}

.sandbox-announce {
  position : absolute;
  width    : 1px;
  height   : 1px;
  margin   : 0;
  overflow : hidden;
  clip     : rect(0 0 0 0);
  white-space : nowrap;
}

.sandbox-deck {
  position      : relative;
  z-index       : 20;
  margin-bottom : 1.5rem;
  background    : var(--vp-c-bg-soft);
}

.sandbox-deck[data-locked='true'] {
  position : sticky;
  top      : var(--vp-nav-height);
}

.sandbox-deck-lock {
  position   : absolute;
  top        : 4px;
  right      : 8px;
  z-index    : 1;
  padding    : 6px;
  border     : 0;
  background : transparent;
  color      : var(--vp-c-text-3);
  cursor     : pointer;
  transition : color var(--prose-transition);
}

.sandbox-deck-lock:hover {
  color : var(--vp-c-text-1);
}

.sandbox-deck-lock[aria-pressed='true'] {
  color : var(--vp-c-brand-1);
}

.sandbox-deck-lock:focus-visible {
  outline        : var(--prose-focus-ring);
  outline-offset : 1px;
}

.sandbox-deck-lock-icon {
  --glyph-size : 15px;
}

.sandbox-deck-body {
  padding : 0.75rem 1.25rem 1.25rem;
}

.sandbox-deck-head {
  display         : flex;
  align-items     : center;
  justify-content : center;
  width           : 100%;
  padding         : 0.45rem;
  border          : 0;
  background      : transparent;
  color           : var(--vp-c-text-2);
  cursor          : pointer;
  transition      : color var(--prose-transition), background var(--prose-transition);
}

.sandbox-deck-head:hover {
  color      : var(--vp-c-text-1);
  background : color-mix(in srgb, var(--vp-c-text-1) 4%, transparent);
}

.sandbox-deck-head[aria-expanded='true'] {
  border-bottom : 1px solid var(--vp-c-divider);
}

.sandbox-deck-head:focus-visible {
  outline        : var(--prose-focus-ring);
  outline-offset : -2px;
}

@media (--prose-bp-tablet) {
  .sandbox-deck[data-locked='true'] {
    position : static;
  }

  .sandbox-deck-lock {
    display : none;
  }

  .sandbox-surfaces {
    grid-template-columns : 1fr;
  }
}

@media (prefers-reduced-motion: reduce) {
  .sandbox-corner {
    transition : none;
  }
}
</style>
