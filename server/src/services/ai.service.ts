// AI 智能搜索 — LLM 分析意图 → 多策略并行 → 合并去重

import { loadConfig } from '../config/index.js'
import { executeStrategies, getAvailableStrategies, type SearchResult } from './search.service.js'

export { type SearchResult }

export async function aiSearch(
  naturalQuery: string
): Promise<SearchResult[]> {
  const term = naturalQuery.trim()
  if (term.length === 0) return []

  const config = (await loadConfig()) as Record<string, unknown>
  const ai = (config.ai ?? {}) as Record<string, string>
  const baseUrl = ai.base_url ?? 'https://api.openai.com/v1'
  const model = ai.model ?? 'gpt-4o-mini'
  const apiKey = ai.api_key ?? ''

  if (!apiKey) throw new Error('未配置 AI API Key，请先在设置中填写')

  const strategies = await selectStrategies(term, baseUrl, model, apiKey)

  return executeStrategies(strategies)
}

async function selectStrategies(
  query: string,
  baseUrl: string,
  model: string,
  apiKey: string
): Promise<Array<{ strategy: string; params: Record<string, string> }>> {
  const strategies = getAvailableStrategies()
  const strategyList = strategies
    .map((s) => `- ${s.name}: ${s.description}`)
    .join('\n')

  const systemPrompt = `你是一个文档搜索助手。根据用户的自然语言查询，从以下搜索策略中选择最匹配的一个或多个。

可用策略：
${strategyList}

返回一个 JSON 数组（即使只选一个也用数组包裹）：
[
  { "strategy": "策略名", "params": { "参数名": "参数值" } }
]

各策略的 params：
- fulltext: { "query": "关键词" }
- category: { "keyword": "分类" }  支持 notes/code/data/finance/identity/journal
- filetype: { "ext": "扩展名" }  如 md/pdf/txt
- summary: { "query": "关键词" }
- recent: { "days": "天数" }  如 7 = 最近 7 天
- privacy: { "folderType": "public" }  或 private
- filename: { "filename": "文件名关键词" }
- tag: { "tag": "标签" }

规则：
- "几天前"/"上周" → recent，有明确主题时也加其他匹配策略
- "代码的教程" → category + fulltext
- "PDF文件" → filetype（+fulltext 如果有关键词）
- "关于XX" → summary
- 单一意图只返回一个策略，不要硬凑
- 多意图返回多个策略，不要重复同一个策略
- TS→TypeScript, JS→JavaScript
- 只返回 JSON 数组，不要 markdown 代码块`

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
    return JSON.parse(content.trim()) as Array<{
      strategy: string
      params: Record<string, string>
    }>
  } catch {
    const match = content.match(/```(?:json)?\s*\n?([\s\S]*?)```/)
    if (match) {
      return JSON.parse(match[1].trim()) as Array<{
        strategy: string
        params: Record<string, string>
      }>
    }
    throw new Error('无法解析 LLM 返回的策略数组')
  }
}
