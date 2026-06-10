import { apiUrl } from './api-client'

export interface AiConfig {
  base_url: string
  model: string
  api_key: string
}

export interface PathsConfig {
  root: string
  library: string
}

export interface ResolvedPathsConfig {
  root: string
  library: string
  database?: string
  runtime_base?: string
  config_path?: string
}

export type ConfigPayload = AiConfig & PathsConfig & {
  resolved_paths?: ResolvedPathsConfig
  database_path?: string
}

interface ConfigResponse {
  ai?: Partial<AiConfig>
  paths?: Partial<PathsConfig>
  _meta?: {
    resolved_paths?: Partial<ResolvedPathsConfig>
    database_path?: string
  }
  [key: string]: unknown
}

function normalizeConfig(raw: unknown): ConfigPayload {
  if (!raw || typeof raw !== 'object') {
    throw new Error('无效的配置响应')
  }

  const data = raw as ConfigResponse
  const ai = data.ai ?? {}
  const paths = data.paths ?? {}

  const resolvedPaths = data._meta?.resolved_paths

  return {
    base_url: typeof ai.base_url === 'string' ? ai.base_url : '',
    model: typeof ai.model === 'string' ? ai.model : '',
    api_key: typeof ai.api_key === 'string' ? ai.api_key : '',
    root: typeof paths.root === 'string' ? paths.root : '',
    library: typeof paths.library === 'string' ? paths.library : '',
    resolved_paths: resolvedPaths
      ? {
          root: typeof resolvedPaths.root === 'string' ? resolvedPaths.root : '',
          library: typeof resolvedPaths.library === 'string' ? resolvedPaths.library : '',
          database: typeof resolvedPaths.database === 'string' ? resolvedPaths.database : undefined,
          runtime_base: typeof resolvedPaths.runtime_base === 'string' ? resolvedPaths.runtime_base : undefined,
          config_path: typeof resolvedPaths.config_path === 'string' ? resolvedPaths.config_path : undefined,
        }
      : undefined,
    database_path: typeof data._meta?.database_path === 'string' ? data._meta.database_path : undefined,
  }
}

export async function fetchConfig(): Promise<ConfigPayload> {
  const response = await fetch(apiUrl('/api/config'), {
    method: 'GET',
    headers: {
      Accept: 'application/json',
    },
  })

  if (!response.ok) {
    const text = await response.text()
    throw new Error(`请求配置失败: ${response.status} ${response.statusText} ${text}`)
  }

  const data = await response.json()
  return normalizeConfig(data)
}

export async function saveConfig(config: ConfigPayload): Promise<void> {
  const payload: Record<string, unknown> = {
    ai: {
      base_url: config.base_url,
      model: config.model,
      api_key: config.api_key,
    },
    paths: {
      root: config.root,
      library: config.library,
    },
  }

  const response = await fetch(apiUrl('/api/config'), {
    method: 'PUT',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: JSON.stringify(payload),
  })

  if (!response.ok) {
    const text = await response.text()
    throw new Error(`保存配置失败: ${response.status} ${response.statusText} ${text}`)
  }
}
