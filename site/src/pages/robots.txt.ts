import type { APIRoute } from 'astro'

// Allow-all crawling, pointing crawlers at the index the sitemap integration
// emits.
export const GET: APIRoute = ({ site }) => {
  const lines = [
    'User-agent: *', 'Allow: /', '',
    `Sitemap: ${new URL('sitemap-index.xml', site)}`, ''
  ]
  return new Response(lines.join('\n'), { headers: { 'Content-Type': 'text/plain' } })
}
