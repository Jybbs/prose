<script setup lang="ts">
import { ref } from 'vue'

const props = defineProps<{ code: string; lang?: string }>()
const copied = ref(false)

async function onCopy() {
  if (typeof navigator === 'undefined' || !navigator.clipboard) return
  await navigator.clipboard.writeText(props.code)
  copied.value = true
  setTimeout(() => { copied.value = false }, 1400)
}
</script>

<template>
  <div class="copy-block">
    <pre><code :class="lang ? `language-${lang}` : undefined">{{ code }}</code></pre>
    <button
      :class="['copy-block-button', { copied }]"
      type="button"
      @click="onCopy"
    >
      {{ copied ? 'Copied' : 'Copy' }}
    </button>
  </div>
</template>
