<script setup lang="ts">
import { onMounted } from 'vue'

import { useProseSandbox } from '../../../lib/composables/use-prose-sandbox'
import { data as seed }    from '../../../lib/sandbox/seed.data'

const { error, format, output, source, status } = useProseSandbox({ source: seed.source })

// The glue imports client-side only, so the first format waits for mount
// rather than running on the static-generation path.
onMounted(format)
</script>

<template>
  <div class="sandbox panel panel-clip">
    <div class="sandbox-pane">
      <label class="sandbox-label" for="sandbox-source">Python</label>
      <textarea
        v-model="source"
        id="sandbox-source"
        class="sandbox-editor"
        autocapitalize="off"
        autocomplete="off"
        autocorrect="off"
        spellcheck="false"
      />
    </div>
    <div class="sandbox-pane sandbox-pane-result">
      <div class="sandbox-label">
        <span>Formatted</span>
        <span v-if="status === 'loading'" class="sandbox-status">Loading…</span>
      </div>
      <pre v-if="error" class="sandbox-result sandbox-result-error">{{ error }}</pre>
      <pre v-else class="sandbox-result">{{ output }}</pre>
    </div>
  </div>
</template>
