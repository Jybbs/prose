<script setup lang="ts">
interface Feature {
  body   : string
  cta    : string
  icon   : string
  link   : string
  number : string
  title  : string
}

const features: Feature[] = [
  {
    body   : 'Equals signs, colons, the <code>import</code> keyword, and match arrows line up across consecutive lines. The eye drops down the column.',
    cta    : 'align-equals',
    icon   : '🪜',
    link   : '/rules/align-equals',
    number : '01',
    title  : 'Alignment'
  },
  {
    body   : 'Dictionaries, lists, and sets expand to one entry per line. Multi-line collections drop their trailing comma. Single-entry contexts skip padding.',
    cta    : 'collection-layout',
    icon   : '🪶',
    link   : '/rules/collection-layout',
    number : '02',
    title  : 'Layout'
  },
  {
    body   : 'Legacy union syntax, loose constants, step-narration comments, and single-use bindings surface as lint diagnostics, never rewrites.',
    cta    : 'single-use-variables',
    icon   : '🧶',
    link   : '/rules/single-use-variables',
    number : '03',
    title  : 'Lint'
  }
]
</script>

<template>
  <div class="landing-section landing-menu">
    <SectionHeading kicker="The Surfaces" heading="Three rule families. Eighteen rules." centered>
      <a href="/rules/" class="landing-small-button">All rules</a>
    </SectionHeading>
    <div class="landing-feature-grid">
      <a v-for="feature in features" :key="feature.title" :href="feature.link" class="landing-feature-card">
        <span class="card-number">— {{ feature.number }}</span>
        <span class="card-icon">{{ feature.icon }}</span>
        <h3>{{ feature.title }}</h3>
        <p v-html="feature.body" />
        <span class="card-link">{{ feature.cta }} →</span>
      </a>
    </div>
  </div>
</template>
