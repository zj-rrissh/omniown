// AI 智能搜索 — LLM 分析意图 → 多策略并行 → 合并去重
// Stage 1: 查询分析（改写 + 提取关键词）
// Stage 2: 策略选择 + JSON Schema 验证

import { loadConfig } from '../config/index.js'
import {
  buildSystemPrompt,
  resolvePromptVariant,
  buildQueryAnalysisPrompt,
  type QueryAnalysisResult,
} from '../prompts/index.js'
import {
  executeStrategiesWithTrace,
  getAvailableStrategies,
  getDocumentStats,
  type SearchResult,
  type StrategyCall,
  type StrategySearchTrace,
} from './search.service.js'
import { validateStrategies } from '../utils/validate-strategies.js'

export { type SearchResult }

export interface AiSearchTrace extends StrategySearchTrace {
  model: string
  baseUrl: string
  prompt: string
  rawResponse: string
  /** Stage 1 查询分析结果（改写 + 关键词提取） */
  queryAnalysis?: QueryAnalysisResult
  /** Stage 2 策略输出验证失败时的错误信息 */
  validationError?: string
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

  // ---- Stage 1: 查询分析 ----
  let queryAnalysis: QueryAnalysisResult | undefined
  let queryForStrategy = term

  try {
    queryAnalysis = await analyzeQuery(term, baseUrl, model, apiKey)
    // 用改写后的查询作为 Stage 2 输入
    queryForStrategy = queryAnalysis.rewrittenQuery
  } catch (err) {
    // Stage 1 失败不阻塞搜索，降级使用原始 query
    console.warn('[ai-search] 查询分析失败，降级使用原始查询:', err instanceof Error ? err.message : err)
  }

  // ---- Stage 2: 策略选择 ----
  let validationError: string | undefined
  let decision: { strategies: StrategyCall[]; rawResponse: string }

  try {
    decision = await selectStrategies(term, queryAnalysis, queryForStrategy, baseUrl, model, apiKey, variant)
  } catch (err) {
    // 如果是策略验证失败，记录错误并抛出（无法降级）
    if (err instanceof Error && err.message.startsWith('策略输出格式验证失败')) {
      validationError = err.message
    }
    throw err
  }

  const { results, trace } = await executeStrategiesWithTrace(decision.strategies)

  return {
    results,
    trace: {
      ...trace,
      model,
      baseUrl,
      prompt: term,
      rawResponse: decision.rawResponse,
      queryAnalysis,
      validationError,
    },
  }
}

// ---- Stage 1: 查询分析 ----

/**
 * Stage 1 — 查询分析
 * 改写用户查询并提取结构化关键词/意图/分类等信息。
 *
 * 失败时抛出异常，由调用方决定降级策略。
 */
async function analyzeQuery(
  query: string,
  baseUrl: string,
  model: string,
  apiKey: string
): Promise<QueryAnalysisResult> {
  const systemPrompt = buildQueryAnalysisPrompt()

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
      max_tokens: 500,
    }),
  })

  if (!response.ok) {
    const body = await response.text()
    throw new Error(`查询分析 LLM 请求失败 (${response.status}): ${body.slice(0, 200)}`)
  }

  const data = (await response.json()) as {
    choices?: Array<{ message?: { content?: string } }>
  }
  const content = data.choices?.[0]?.message?.content
  if (!content) throw new Error('查询分析 LLM 返回内容为空')

  // 解析 JSON（兼容 markdown 代码块包裹）
  try {
    return JSON.parse(content.trim()) as QueryAnalysisResult
  } catch {
    const match = content.match(/```(?:json)?\s*\n?([\s\S]*?)```/)
    if (match) {
      return JSON.parse(match[1].trim()) as QueryAnalysisResult
    }
    throw new Error(`查询分析 JSON 解析失败: ${content.slice(0, 200)}`)
  }
}

// ---- Stage 2: 策略选择 ----

async function selectStrategies(
  originalQuery: string,
  queryAnalysis: QueryAnalysisResult | undefined,
  rewrittenQuery: string,
  baseUrl: string,
  model: string,
  apiKey: string,
  variant?: 'v1' | 'v2'
): Promise<{ strategies: StrategyCall[]; rawResponse: string }> {
  const strategies = getAvailableStrategies()

  // v2：注入文档库统计信息作为 prompt 上下文
  const context =
    variant === 'v2' ? await getDocumentStats() : undefined

  // 从 prompt 模块生成 System Prompt
  const systemPrompt = buildSystemPrompt(strategies, { variant, context })

  // 构建 User Message：原始查询 + Stage 1 分析结果
  const userMessage = queryAnalysis
    ? `原始查询: ${originalQuery}\n改写查询: ${rewrittenQuery}\n分析结果: ${JSON.stringify(queryAnalysis)}`
    : originalQuery

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
        { role: 'user', content: userMessage },
      ],
      temperature: 0.1,
      max_tokens: 1000,
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

  // 解析 + zod 验证 LLM 输出
  let parsed: unknown
  try {
    parsed = JSON.parse(content.trim())
  } catch {
    const match = content.match(/```(?:json)?\s*\n?([\s\S]*?)```/)
    if (match) {
      parsed = JSON.parse(match[1].trim())
    } else {
      throw new Error(`无法解析 LLM 返回的策略数组: ${content.slice(0, 200)}`)
    }
  }

  return {
    strategies: validateStrategies(parsed),
    rawResponse: content,
  }
}
