<script setup lang="ts">
import { refAutoReset, useClipboard, useStorage } from '@vueuse/core'
import { computed, onMounted, ref }               from 'vue'

import CopyButton from '../base/CopyButton.vue'

import { useProseSandbox } from '../../../lib/composables/use-prose-sandbox'
import { data as schema }  from '../../../lib/sandbox/config-schema.data'
import { data as pool }    from '../../../lib/sandbox/pool.data'

const deckLocked   = useStorage('prose-sandbox-pinned', false)
const deckOpen     = useStorage('prose-sandbox-deck-open', true)
const editing      = ref(false)
const guide        = ref<{ hue: string, value: number } | null>(null)
const refreshArmed = refAutoReset(false, 4000)
const sandbox      = useProseSandbox({ cases: pool.cases, schema })

function onDragging(key: string, hue: string): void {
  guide.value = key ? { hue, value: sandbox.lengthValue(key) } : null
}

function onPreview(_key: string, value: number): void {
  if (guide.value) guide.value = { ...guide.value, value }
}

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
  <div class="sandbox" :data-editing="editing || null">
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
          v-model:editing="editing"
          :sandbox="sandbox"
          :guide="guide?.value ?? null"
          :guide-hue="guide?.hue"
        />
        <CopyButton v-show="!editing" label="Copy the formatted Python" :source="sandbox.formatted.value" />
        <button
          v-show="!editing"
          type="button"
          class="panel-seat panel-corner sandbox-refresh"
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
          v-show="!editing"
          type="button"
          class="panel-seat panel-corner sandbox-share"
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
