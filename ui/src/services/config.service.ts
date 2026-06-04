export interface AiConfig {
  base_url: string
  model: string
  api_key: string
}

export interface PathsConfig {
  root: string
  library: string
}

export type ConfigPayload = AiConfig & PathsConfig

interface ConfigResponse {
  ai?: Partial<AiConfig>
  paths?: Partial<PathsConfig>
  [key: string]: unknown
}

function normalizeConfig(raw: unknown): ConfigPayload {
  if (!raw || typeof raw !== 'object') {
    throw new Error('无效的配置响应')
  }

  const data = raw as ConfigResponse
  const ai = data.ai ?? {}
  const paths = data.paths ?? {}

  return {
    base_url: typeof ai.base_url === 'string' ? ai.base_url : '',
    model: typeof ai.model === 'string' ? ai.model : '',
    api_key: typeof ai.api_key === 'string' ? ai.api_key : '',
    root: typeof paths.root === 'string' ? paths.root : '',
    library: typeof paths.library === 'string' ? paths.library : '',
  }
}

export async function fetchConfig(): Promise<ConfigPayload> {
  const response = await fetch('/api/config', {
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

  const response = await fetch('/api/config', {
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
