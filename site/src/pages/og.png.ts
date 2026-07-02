import type { APIRoute } from 'astro'

import { cardResponse } from '../lib/og/render'

export const GET: APIRoute = () => cardResponse('index')
