<template>
  <section class="page-spine" aria-label="Install with release chronology">
    <aside class="page-spine-rail" aria-label="Release chronology">
      <p class="kicker" data-variant="rule">On the shelf</p>
      <div class="page-spine-rail-scroll">
        <PyPIReleases />
      </div>
    </aside>

    <div class="page-spine-body">
      <p class="kicker" data-variant="rule">Read on</p>
      <p class="page-spine-lede">
        Take <em>Prose</em> to your own pages and make the next save <em>legible</em>.
      </p>

      <div class="page-spine-row">
        <div class="page-spine-cmd" aria-label="Install command">
          <span class="page-spine-prompt" aria-hidden="true">$</span>
          <code>uv tool install prose-formatter</code>
        </div>
        <a class="page-spine-primary" href="/guide/quick-start">
          <span>Quick start</span>
          <span class="page-spine-arrow" aria-hidden="true">&rarr;</span>
        </a>
      </div>
    </div>
  </section>
</template>
