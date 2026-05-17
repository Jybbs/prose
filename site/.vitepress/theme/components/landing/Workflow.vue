<script setup lang="ts">
interface Step {
  body   : string
  code   : string
  number : string
  title  : string
}

const steps: Step[] = [
  {
    body   : 'Fetch the wheel and expose the <code>prose</code> binary.',
    code   : 'uv tool install prose-formatter',
    number : '01',
    title  : 'Install'
  },
  {
    body   : 'Drop a <code>[tool.prose]</code> table into <code>pyproject.toml</code>. The defaults already work.',
    code   : '[tool.prose]\ntarget-version = "3.13"',
    number : '02',
    title  : 'Configure'
  },
  {
    body   : 'Rewrite in place, or check without writing.',
    code   : 'prose format path/\nprose check path/',
    number : '03',
    title  : 'Run'
  },
  {
    body   : 'Pair with Ruff as the token-level upstream pass.',
    code   : 'ruff format && prose format',
    number : '04',
    title  : 'Compose'
  }
]
</script>

<template>
  <div class="landing-section landing-quickstart">
    <div class="landing-section-heading centered">
      <p class="landing-kicker">The Workflow</p>
      <h2>Four lines from install to running.</h2>
    </div>
    <div class="landing-workflow-grid">
      <div v-for="step in steps" :key="step.number" class="workflow-step">
        <span class="step-number">— {{ step.number }}</span>
        <h3>{{ step.title }}</h3>
        <p v-html="step.body" />
        <pre><code>{{ step.code }}</code></pre>
      </div>
    </div>
  </div>
</template>
