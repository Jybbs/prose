<script setup lang="ts">
import Fixture  from '../fixtures/Fixture.vue'
import RuleChip from './RuleChip.vue'

import { data as composition } from '../../../data/composition.data'
import { data as rules }       from '../../../data/rules.data'
</script>

<template>
  <div class="composition-grid">
    <section v-for="entry in composition.cases" :key="entry.case" class="composition-case">
      <header class="composition-case-header">
        <h3 :id="entry.case" class="composition-case-title">{{ entry.title }}</h3>
        <ul class="composition-case-rules">
          <li v-for="slug in entry.rules" :key="slug">
            <RuleChip v-if="rules.bySlug[slug]" :slug="slug" />
            <code v-else>{{ slug }}</code>
          </li>
        </ul>
      </header>
      <Fixture rule="composition" :case="entry.case" :title="entry.title" />
    </section>
  </div>
</template>

<style scoped>
.composition-grid {
  margin-top: 32px;
}

.composition-case + .composition-case {
  margin-top    : 48px;
  padding-top   : 36px;
  border-top    : 1px solid var(--vp-c-divider);
}

.composition-case-header {
  margin-bottom: 20px;
}

.composition-case-title {
  margin      : 0 0 10px;
  font-family : var(--vp-font-family-base);
  font-size   : 1.3rem;
  font-weight : 500;
  color       : var(--vp-c-text-1);
}

.composition-case-rules {
  list-style : none;
  padding    : 0;
  margin     : 0;
  display    : flex;
  flex-wrap  : wrap;
  gap        : 6px;
}

.composition-case-rules li {
  margin: 0;
}
</style>
