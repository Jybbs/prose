import type { APIRoute, GetStaticPaths } from 'astro'

import { cardResponse, pageCardIds } from '../../lib/og/render'

export const getStaticPaths: GetStaticPaths = async () =>
  (await pageCardIds()).map(id => ({ params: { slug: id } }))

export const GET: APIRoute = ({ params }) => cardResponse(params.slug!)
