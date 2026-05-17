<script setup lang="ts">
import { ref } from 'vue'

interface ExitCode {
  code     : number
  detail   : string[]
  label    : string
  summary  : string
}

const codes: ExitCode[] = [
  {
    code   : 0,
    label  : 'Clean',
    summary: 'No diagnostics, no rewrites pending.',
    detail : [
      'Returned by both `prose check` and `prose format` when the input is already conforming.',
      'CI gates pass without further work.'
    ]
  },
  {
    code   : 1,
    label  : 'Format would change',
    summary: 'At least one auto-fix diagnostic is pending.',
    detail : [
      '`prose check` returns this when one or more auto-fix rules would emit edits.',
      '`prose format` suppresses this code (returns 0) when the rewrite succeeds, since the changes were applied rather than left pending.',
      'Every auto-fix rule contributes here.'
    ]
  },
  {
    code   : 2,
    label  : 'Lint violation',
    summary: 'At least one lint-only diagnostic surfaced.',
    detail : [
      'Surfaces under both `prose check` and `prose format`.',
      'The four shipped lints contribute: `legacy-union-syntax`, `loose-constants`, `no-step-narration`, `single-use-variables`.'
    ]
  },
  {
    code   : 3,
    label  : 'Parse error',
    summary: 'Input could not be parsed as Python.',
    detail : [
      'Surfaces under both subcommands when `ruff_python_parser` fails on the source.',
      'The pipeline does not run; no other diagnostics fire.'
    ]
  },
  {
    code   : 4,
    label  : 'Config error',
    summary: 'pyproject.toml or argument validation failed.',
    detail : [
      'Surfaces from `Config::from_pyproject_str` errors, malformed `--select` / `--ignore` flags, or unknown CLI options.',
      'Pre-empts every other code (the run never reaches the pipeline).'
    ]
  }
]

const active = ref<number | null>(null)
const toggle = (n: number) => { active.value = active.value === n ? null : n }
</script>

<template>
  <div class="exit-codes">
    <div
      v-for="row in codes"
      :key="row.code"
      :class="['exit-code-row', { active: active === row.code }]"
      role="button"
      tabindex="0"
      @click="toggle(row.code)"
      @keydown.enter="toggle(row.code)"
      @keydown.space.prevent="toggle(row.code)"
    >
      <div class="exit-code-number">{{ row.code }}</div>
      <div class="exit-code-body">
        <div class="exit-code-label">{{ row.label }}</div>
        <p class="exit-code-summary">{{ row.summary }}</p>
        <div v-if="active === row.code" class="exit-code-detail">
          <ul>
            <li v-for="(line, idx) in row.detail" :key="idx">{{ line }}</li>
          </ul>
        </div>
      </div>
    </div>
  </div>
</template>
