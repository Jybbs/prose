// @ts-check
import { defineConfig }        from 'astro/config';
import starlight               from '@astrojs/starlight';
import starlightLinksValidator from 'starlight-links-validator';

export default defineConfig({
  site         : 'https://prose.fyi',
  integrations : [
    starlight({
      title   : 'Prose',
      plugins : [starlightLinksValidator()],
    }),
  ],
});
