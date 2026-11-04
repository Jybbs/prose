<script setup lang="ts">
import { ref } from 'vue'

import { data as releases } from '../../../lib/landing/pypi-releases.data'
import { PYPI_PACKAGE }     from '../../../lib/shared/constants'

import CopyButton    from '../base/CopyButton.vue'
import ReleaseLedger from './ReleaseLedger.vue'
import ReleaseStamp  from './ReleaseStamp.vue'
import ReleaseToggle from './ReleaseToggle.vue'

const current = releases[0]
const extras  = releases.slice(1)
const open    = ref(false)

const installCmd = `uv tool install ${PYPI_PACKAGE}`
</script>

<template>
  <section class="landing-cta relative mt-76 grid grid-cols-196-minmax-1fr overflow-hidden border-1 border-divider bg-bg-soft text-left max-card-stack:grid-cols-1fr">
    <aside class="relative min-h-200 overflow-hidden border-r-1 border-r-panel-edge bg-panel-bg max-card-stack:border-b-1 max-card-stack:border-r-0 max-card-stack:border-r-current max-card-stack:border-b-panel-edge">
      <Transition name="landing-cta-swap" mode="out-in" enter-from-class="translate-y-8 opacity-0" leave-to-class="translate-y-8 opacity-0">
        <div v-if="!open" key="stamp" class="absolute inset-0 flex flex-col items-center justify-center px-14 py-16">
          <ReleaseStamp :release="current" />
        </div>
        <div v-else key="open" class="absolute inset-0 flex flex-col items-center justify-center px-14 py-16">
          <ReleaseLedger :releases="extras" />
        </div>
      </Transition>
      <ReleaseToggle :open="open" @toggle="open = !open" />
    </aside>

    <div class="relative grid grid-cols-minmax-1fr-auto items-center gap-x-18 gap-y-4 px-22 py-18">
      <p class="col-start-1 col-end-3 kicker mb-4">Read on</p>
      <p class="col-start-1 col-end-3 mb-10 max-w-48ch font-display text-1-25rem leading-1-3 font-400 text-text-1">
        Bring <em class="text-brand-1"><span class="text-ube">Prose</span></em> to your own pages and make the next save <em class="text-brand-1">legible</em>.
      </p>
      <div class="landing-cta-cmd copy-host relative col-1 inline-flex items-center gap-10 rounded-sm border-1 border-divider bg-bg py-8 pr-44 pl-14 font-mono text-code text-text-1" aria-label="Install command">
        <span class="text-text-3" aria-hidden="true">$</span>
        <code>{{ installCmd }}</code>
        <CopyButton label="Copy install command" :source="installCmd" :copied-during="2000" />
      </div>
      <a class="group col-2 inline-flex items-center gap-6 rounded-sm bg-brand-1 px-16 py-8 font-display text-md font-500 text-bg italic transition-colors hover:bg-brand-2" href="/guide/quick-start">
        <span>Quick start</span>
        <span class="transition-transform group-hover:translate-x-3" aria-hidden="true">→</span>
      </a>
    </div>
  </section>
</template>
