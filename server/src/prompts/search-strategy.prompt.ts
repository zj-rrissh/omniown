/**
 * AI 搜索策略选择 — System Prompt 模板
 *
 * 支持多版本 prompt，通过 omniown.toml [ai].prompt_variant 切换。
 * 新增 variant 只需在此文件添加，无需修改业务代码。
 */

// ---- 类型定义 ----

export interface StrategyMeta {
  name: string
  description: string
}

export type PromptVariant = 'v1' | 'v2'

// ---- 公共辅助 ----

function buildStrategyList(strategies: StrategyMeta[]): string {
  return strategies.map((s) => `- ${s.name}: ${s.description}`).join('\n')
}

function buildParamDocs(): string {
  return [
    '各策略的 params：',
    '- fulltext: { "query": "关键词" }',
    '- category: { "keyword": "分类" } 支持 notes/code/data/finance/identity/journal',
    '- filetype: { "ext": "扩展名" } 如 md/pdf/txt',
    '- summary: { "query": "关键词" }',
    '- recent: { "days": "天数" } 如 7 = 最近 7 天',
    '- privacy: { "folderType": "public" } 或 private',
    '- filename: { "filename": "文件名关键词" }',
    '- tag: { "tag": "标签" }',
  ].join('\n')
}

function buildOutputRules(): string {
  return [
    '规则：',
    '- "几天前"/"上周" → recent，有明确主题时也加其他匹配策略',
    '- "代码的教程" → category + fulltext',
    '- "PDF文件" → filetype（+fulltext 如果有关键词）',
    '- "关于XX" → summary',
    '- 单一意图只返回一个策略，不要硬凑',
    '- 多意图返回多个策略，不要重复同一个策略',
    '- TS→TypeScript, JS→JavaScript',
    '- 只返回 JSON 数组，不要 markdown 代码块',
  ].join('\n')
}

// ---- Prompt Variant: v1（当前默认，与原硬编码一致） ----

function buildPromptV1(strategies: StrategyMeta[]): string {
  return `你是一个文档搜索助手。根据用户的自然语言查询，从以下搜索策略中选择最匹配的一个或多个。

可用策略：
${buildStrategyList(strategies)}

返回一个 JSON 数组（即使只选一个也用数组包裹）：
[
  { "strategy": "策略名", "params": { "参数名": "参数值" } }
]

${buildParamDocs()}

${buildOutputRules()}`
}

// ---- Prompt Variant: v2（增强版 — Few-shot 示例 + 文档库上下文注入） ----

function buildPromptV2(
  strategies: StrategyMeta[],
  context?: { totalDocs?: number; categories?: string[] }
): string {
  const contextBlock = context?.totalDocs
    ? [
        '',
        '【用户文档库信息】',
        `文档总数: ${context.totalDocs}`,
        context.categories?.length
          ? `已知分类: ${context.categories.join(', ')}`
          : '',
        '优先根据上述信息判断哪些策略更可能命中结果。',
      ].filter(Boolean).join('\n')
    : ''

  return `你是一个文档搜索助手。根据用户的自然语言查询，从以下搜索策略中选择最匹配的一个或多个。

【可用策略】
${buildStrategyList(strategies)}
${contextBlock}

【输出格式】
返回纯 JSON 数组（不要 markdown 代码块）：
[
  { "strategy": "策略名", "params": { "参数名": "参数值" } }
]

${buildParamDocs()}

【Few-shot 示例】
用户: "我上周写的机器学习笔记"
输出: [{ "strategy": "recent", "params": { "days": "7" } }, { "strategy": "category", "params": { "keyword": "notes" } }, { "strategy": "fulltext", "params": { "query": "机器学习" } }]

用户: "所有PDF文件"
输出: [{ "strategy": "filetype", "params": { "ext": "pdf" } }]

用户: "关于Docker的代码教程"
输出: [{ "strategy": "category", "params": { "keyword": "code" } }, { "strategy": "fulltext", "params": { "query": "Docker" } }]

用户: "私密的财务数据"
输出: [{ "strategy": "privacy", "params": { "folderType": "private" } }, { "strategy": "category", "params": { "keyword": "finance" } }]

用户: "最近3天的日记"
输出: [{ "strategy": "recent", "params": { "days": "3" } }, { "strategy": "fulltext", "params": { "query": "日记" } }]

【规则】
- "几天前"/"上周"/"最近" → recent 策略
- "代码"/"笔记"/"财务"/"日记" → category 策略
- "PDF"/"Markdown"/"图片" → filetype 策略
- "公开的"/"私密的" → privacy 策略
- 有明确搜索关键词时同时加 fulltext 策略
- 单一意图只返回一个策略，不要硬凑
- 多意图返回多个策略（每类一个），不要重复同一个策略
- TS→TypeScript, JS→JavaScript
- 如果完全无法判断意图，返回 [{ "strategy": "fulltext", "params": { "query": "用户原始输入" } }]
- 只返回 JSON 数组，不要 markdown 代码块、不要解释文字
- 策略数组不能为空`
}

// ---- 导出入口 ----

export interface BuildPromptOptions {
  /** Prompt 版本，默认读取配置 */
  variant?: PromptVariant
  /** 用户文档库统计信息（v2 使用） */
  context?: { totalDocs?: number; categories?: string[] }
}

/**
 * 根据 variant 生成对应的 System Prompt。
 *
 * @param strategies - 可用策略列表（来自 search.service.ts）
 * @param options - Prompt 构建选项
 * @returns System Prompt 字符串
 */
export function buildSystemPrompt(
  strategies: StrategyMeta[],
  options: BuildPromptOptions = {}
): string {
  const variant = options.variant ?? 'v1'

  if (variant === 'v2') {
    return buildPromptV2(strategies, options.context)
  }

  // 默认 v1
  return buildPromptV1(strategies)
}

/**
 * 从 omniown.toml [ai] 配置中读取 prompt_variant。
 * 供业务代码调用，保持配置读取逻辑集中。
 */
export function resolvePromptVariant(
  aiConfig?: Record<string, string>
): PromptVariant {
  const variant = aiConfig?.prompt_variant
  if (variant === 'v1' || variant === 'v2') return variant
  return 'v1'
}
