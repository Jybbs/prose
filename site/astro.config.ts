import { defineConfig }        from 'astro/config';
import starlight               from '@astrojs/starlight';
import starlightLinksValidator from 'starlight-links-validator';

import { markdownConfig } from './src/lib/markdown/config';

export default defineConfig({
  site     : 'https://prose.fyi',
  markdown : markdownConfig,
  integrations: [
    starlight({
      title   : 'Prose',
      plugins : [starlightLinksValidator()],
    }),
  ],
});
