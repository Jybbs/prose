<template>
  <section class="landing-cta" aria-label="Install with release chronology">
    <aside class="landing-cta-rail" aria-label="Release chronology">
      <p class="landing-cta-kicker">On the shelf</p>
      <div class="landing-cta-rail-scroll">
        <PyPIReleases />
      </div>
    </aside>

    <div class="landing-cta-body">
      <p class="landing-cta-kicker">Read on</p>
      <p class="landing-cta-lede">
        Take <em>Prose</em> to your own pages and make the next save <em>legible</em>.
      </p>

      <div class="landing-cta-cmd" aria-label="Install command">
        <span class="landing-cta-prompt" aria-hidden="true">$</span>
        <code>uv tool install prose-formatter</code>
      </div>

      <a class="landing-cta-primary" href="/guide/quick-start">
        <span>Quick start</span>
        <span class="landing-cta-arrow" aria-hidden="true">→</span>
      </a>
    </div>
  </section>
</template>
