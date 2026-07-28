<script setup lang="ts">
import { fixtureEntry } from '../../../lib/fixtures/entry'
import InlineProse      from '../base/InlineProse.vue'

interface Run {
  badge : 'changed' | 'settled'
  html ?: string
  label : string
}

const props = defineProps<{
  case : string
  rule : string
}>()

const entry = fixtureEntry(props.rule, props.case)

// A fixture that rewrites its input settles on the run after the one that
// changed it, whereas a fixed-point fixture is already settled on run one.
const runs: Run[] = entry.changesSource
  ? [
      { badge: 'changed', html: entry.outputHtml, label: 'Run 1' },
      { badge: 'settled', label: 'Run 2' }
    ]
  : [{ badge: 'settled', label: 'Run 1' }]
</script>

<template>
  <section class="fixture-converge">
    <div v-if="entry.descriptionNodes" class="fixture-converge-lead">
      <InlineProse :nodes="entry.descriptionNodes" />
    </div>
    <ol class="fixture-converge-track">
      <li class="fixture-converge-step">
        <p class="fixture-converge-mark">
          <span class="fixture-converge-run">As written</span>
        </p>
        <div class="fixture-converge-state panel panel-clip" v-html="entry.inputHtml" />
      </li>
      <li
        v-for="run in runs"
        :key="run.label"
        class="fixture-converge-step"
        :data-badge="run.badge"
      >
        <p class="fixture-converge-mark">
          <span class="fixture-converge-run">{{ run.label }}</span>
          <span class="fixture-converge-badge">
            {{ run.badge === 'changed' ? 'Rewritten' : 'No change' }}
          </span>
        </p>
        <div
          v-if="run.html"
          class="fixture-converge-state panel panel-clip"
          v-html="run.html"
        />
        <p v-else class="fixture-converge-note">
          This run reads the previous output and rewrites nothing, so the file has reached its
          fixed point and every later run leaves it exactly as it stands.
        </p>
      </li>
    </ol>
  </section>
</template>
