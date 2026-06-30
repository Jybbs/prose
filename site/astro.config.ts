import { defineConfig }        from 'astro/config';
import sitemap                 from '@astrojs/sitemap';
import starlight               from '@astrojs/starlight';
import starlightLinksValidator from 'starlight-links-validator';

import { buildContentTimestamps, lastmodForUrl } from './src/lib/config/page-timestamps';
import { watchCrateSources }                     from './src/lib/integrations/watch-crate';
import { markdownConfig }                        from './src/lib/markdown/config';

const siteRoot   = new URL('./', import.meta.url);
const timestamps = buildContentTimestamps(siteRoot);

export default defineConfig({
  site     : 'https://prose.fyi',
  markdown : markdownConfig,
  integrations: [
    starlight({
      title       : 'Prose',
      lastUpdated : true,
      plugins     : [starlightLinksValidator()],
    }),
    sitemap({
      serialize(item) {
        const lastmod = lastmodForUrl(item.url, timestamps);
        return lastmod ? { ...item, lastmod } : item;
      },
    }),
    watchCrateSources(),
  ],
});
