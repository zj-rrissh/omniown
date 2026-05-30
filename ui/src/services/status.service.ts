export interface StatusResponse {
  database: string
  root: string
  schema: {
    current_version: number
    pending_migrations: number
  }
  documents: {
    total: number
    public: number
    private: number
    indexed: number
    failed: number
  }
}

import { getJson, withContext } from './api-client'

export function fetchStatus(): Promise<StatusResponse> {
  return getJson<StatusResponse>('/api/status').catch((error: unknown) => {
    throw withContext(error, 'Failed to load status')
  })
}