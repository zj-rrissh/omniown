// AI 智能搜索 — LLM 分析意图 → 多策略并行 → 合并去重

import { loadConfig } from '../config/index.js'
import {
  buildSystemPrompt,
  resolvePromptVariant,
} from '../prompts/index.js'
import {
  executeStrategiesWithTrace,
  getAvailableStrategies,
  type SearchResult,
  type StrategyCall,
  type StrategySearchTrace,
} from './search.service.js'

export { type SearchResult }

export interface AiSearchTrace extends StrategySearchTrace {
  model: string
  baseUrl: string
  prompt: string
  rawResponse: string
}

export async function aiSearch(
  naturalQuery: string
): Promise<SearchResult[]> {
  const { results } = await aiSearchWithTrace(naturalQuery)
  return results
}

export async function aiSearchWithTrace(
  naturalQuery: string
): Promise<{ results: SearchResult[]; trace: AiSearchTrace }> {
  const term = naturalQuery.trim()
  if (term.length === 0) {
    return {
      results: [],
      trace: {
        model: '',
        baseUrl: '',
        prompt: '',
        rawResponse: '',
        selectedStrategies: [],
        strategyResults: [],
        mergedResultCount: 0,
      },
    }
  }

  const config = (await loadConfig()) as Record<string, unknown>
  const ai = (config.ai ?? {}) as Record<string, string>
  const baseUrl = ai.base_url ?? 'https://api.openai.com/v1'
  const model = ai.model ?? 'gpt-4o-mini'
  const apiKey = ai.api_key ?? ''

  if (!apiKey) throw new Error('未配置 AI API Key，请先在设置中填写')

  // 读取 prompt variant 配置，支持 A/B 测试
  const variant = resolvePromptVariant(ai)

  const decision = await selectStrategies(term, baseUrl, model, apiKey, variant)
  const { results, trace } = await executeStrategiesWithTrace(decision.strategies)

  return {
    results,
    trace: {
      ...trace,
      model,
      baseUrl,
      prompt: term,
      rawResponse: decision.rawResponse,
    },
  }
}

async function selectStrategies(
  query: string,
  baseUrl: string,
  model: string,
  apiKey: string,
  variant?: 'v1' | 'v2'
): Promise<{ strategies: StrategyCall[]; rawResponse: string }> {
  const strategies = getAvailableStrategies()

  // 从 prompt 模块生成 System Prompt
  const systemPrompt = buildSystemPrompt(strategies, { variant })

  const response = await fetch(`${baseUrl}/chat/completions`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${apiKey}`,
    },
    body: JSON.stringify({
      model,
      messages: [
        { role: 'system', content: systemPrompt },
        { role: 'user', content: query },
      ],
      temperature: 0.1,
      max_tokens: 400,
    }),
  })

  if (!response.ok) {
    const body = await response.text()
    throw new Error(`LLM 请求失败 (${response.status}): ${body.slice(0, 200)}`)
  }

  const data = (await response.json()) as {
    choices?: Array<{ message?: { content?: string } }>
  }
  const content = data.choices?.[0]?.message?.content
  if (!content) throw new Error('LLM 返回内容为空')

  try {
    return {
      strategies: JSON.parse(content.trim()) as StrategyCall[],
      rawResponse: content,
    }
  } catch {
    const match = content.match(/```(?:json)?\s*\n?([\s\S]*?)```/)
    if (match) {
      return {
        strategies: JSON.parse(match[1].trim()) as StrategyCall[],
        rawResponse: content,
      }
    }
    throw new Error('无法解析 LLM 返回的策略数组')
  }
}
