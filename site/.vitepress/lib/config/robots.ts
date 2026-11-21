import { SITE_HOSTNAME } from '../shared/constants.ts'

export const ROBOTS_TXT = `User-agent: *\nAllow: /\n\nSitemap: ${SITE_HOSTNAME}/sitemap.xml\n`
