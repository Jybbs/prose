<script setup lang="ts">
import { ref } from 'vue'

import { data as releases } from '../../../lib/landing/pypi-releases.data'

import CopyButton    from '../base/CopyButton.vue'
import ReleaseLedger from './ReleaseLedger.vue'
import ReleaseStamp  from './ReleaseStamp.vue'
import ReleaseToggle from './ReleaseToggle.vue'

const current = releases[0]
const extras  = releases.slice(1)
const open    = ref(false)

const installCmd = 'uv tool install prose-formatter'
</script>

<template>
  <section class="landing-cta" :class="{ 'is-open': open }">
    <aside class="landing-cta-panel">
      <Transition name="landing-cta-swap" mode="out-in">
        <div v-if="!open" key="stamp" class="landing-cta-face">
          <ReleaseStamp :release="current" />
        </div>
        <div v-else key="open" class="landing-cta-face landing-cta-open">
          <ReleaseLedger :releases="extras" />
        </div>
      </Transition>
      <ReleaseToggle :open="open" @toggle="open = !open" />
    </aside>

    <div class="landing-cta-body">
      <p class="landing-cta-kicker kicker">Read on</p>
      <p class="landing-cta-lede">
        Bring <em><span class="prose-mark">Prose</span></em> to your own pages and make the next save <em>legible</em>.
      </p>
      <div class="landing-cta-cmd copy-host" aria-label="Install command">
        <span class="landing-cta-prompt" aria-hidden="true">$</span>
        <code>{{ installCmd }}</code>
        <CopyButton label="Copy install command" :source="installCmd" :copied-during="2000" />
      </div>
      <a class="landing-cta-primary" href="/usage/quick-start">
        <span>Quick start</span>
        <span class="landing-cta-arrow" aria-hidden="true">→</span>
      </a>
    </div>
  </section>
</template>
