/**
 * AI Search Strategy Selection — System Prompt Templates
 *
 * Supports multi-version prompts, switchable via omniown.toml [ai].prompt_variant.
 * Add new variants here without modifying business logic.
 */

// ---- Type Definitions ----

export interface StrategyMeta {
  name: string
  description: string
}

export type PromptVariant = 'v1' | 'v2'

// ---- Shared Helpers ----

function buildStrategyList(strategies: StrategyMeta[]): string {
  return strategies.map((s) => `- ${s.name}: ${s.description}`).join('\n')
}

function buildParamDocs(): string {
  return [
    'Strategy params:',
    '- fulltext: { "query": "keyword" }',
    '- category: { "keyword": "category" } — supports notes/code/data/finance/identity/journal',
    '- filetype: { "ext": "extension" } — e.g. md/pdf/txt',
    '- summary: { "query": "keyword" }',
    '- recent: { "days": "N" } — e.g. 7 = last 7 days',
    '- privacy: { "folderType": "public" } or "private"',
    '- filename: { "filename": "filename keyword" }',
    '- tag: { "tag": "tag name" }',
  ].join('\n')
}

function buildOutputRules(): string {
  return [
    'Rules:',
    '- "a few days ago" / "last week" → recent, combine with other strategies if topic keywords present',
    '- "code tutorial for X" → category + fulltext',
    '- "PDF files" → filetype (+ fulltext if keywords present)',
    '- "about X" / "regarding X" → summary',
    '- Single intent → return one strategy only, do not force extras',
    '- Multiple intents → return multiple strategies, do not repeat the same strategy',
    '- TS→TypeScript, JS→JavaScript',
    '- Output raw JSON array only, no markdown code blocks',
  ].join('\n')
}

// ---- Prompt Variant: v1 (default, kept for A/B testing) ----

function buildPromptV1(strategies: StrategyMeta[]): string {
  return `You are a document search assistant. Based on the user's natural language query, select the most appropriate search strategies from the list below.

Available strategies:
${buildStrategyList(strategies)}

Return a JSON array (even for a single strategy, wrap in an array):
[
  { "strategy": "strategy_name", "params": { "param_name": "param_value" } }
]

${buildParamDocs()}

${buildOutputRules()}`
}

// ---- Prompt Variant: v2 (enhanced — Few-shot examples + document library context) ----

function buildPromptV2(
  strategies: StrategyMeta[],
  context?: { totalDocs?: number; categories?: string[] }
): string {
  const contextBlock = context?.totalDocs
    ? [
        '',
        '[Document Library Info]',
        `Total documents: ${context.totalDocs}`,
        context.categories?.length
          ? `Known categories: ${context.categories.join(', ')}`
          : '',
        'Prefer strategies likely to match based on the above info.',
      ].filter(Boolean).join('\n')
    : ''

  return `You are a document search assistant. Based on the user's natural language query, select the most appropriate search strategies from the list below.

[Available Strategies]
${buildStrategyList(strategies)}

[Output Format]
Return a raw JSON array only (no markdown code blocks):
[
  { "strategy": "strategy_name", "params": { "param_name": "param_value" } }
]

${buildParamDocs()}

[Few-shot Examples]
User: "我上周写的机器学习笔记"
Output: [{ "strategy": "recent", "params": { "days": "7" } }, { "strategy": "category", "params": { "keyword": "notes" } }, { "strategy": "fulltext", "params": { "query": "机器学习" } }]

User: "所有PDF文件"
Output: [{ "strategy": "filetype", "params": { "ext": "pdf" } }]

User: "关于Docker的代码教程"
Output: [{ "strategy": "category", "params": { "keyword": "code" } }, { "strategy": "fulltext", "params": { "query": "Docker" } }]

User: "私密的财务数据"
Output: [{ "strategy": "privacy", "params": { "folderType": "private" } }, { "strategy": "category", "params": { "keyword": "finance" } }]

User: "最近3天的日记"
Output: [{ "strategy": "recent", "params": { "days": "3" } }, { "strategy": "fulltext", "params": { "query": "日记" } }]

User: "find Kubernetes deployment configs from last month"
Output: [{ "strategy": "recent", "params": { "days": "30" } }, { "strategy": "fulltext", "params": { "query": "Kubernetes deployment" } }]

${contextBlock}

[Rules]
- "a few days ago" / "last week" / "最近" → use recent strategy
- "code" / "notes" / "finance" / "diary" → use category strategy
- "PDF" / "Markdown" / "images" → use filetype strategy
- "public" / "private" / "公开的" / "私密的" → use privacy strategy
- Whenever there are clear search keywords, also add fulltext strategy
- Single intent → return one strategy only, do not force extras
- Multiple intents → return multiple strategies (one per type), do not repeat the same strategy
- TS→TypeScript, JS→JavaScript
- If intent is completely unclear, fall back to: [{ "strategy": "fulltext", "params": { "query": "user's original input" } }]
- Output raw JSON array only — no markdown code blocks, no explanatory text
- Strategy array must not be empty`
}

// ---- Exports ----

export interface BuildPromptOptions {
  /** Prompt version, defaults to config value */
  variant?: PromptVariant
  /** Document library stats (used by v2) */
  context?: { totalDocs?: number; categories?: string[] }
}

/**
 * Generate the System Prompt for the given variant.
 *
 * @param strategies - Available strategies (from search.service.ts)
 * @param options - Prompt build options
 * @returns System Prompt string
 */
export function buildSystemPrompt(
  strategies: StrategyMeta[],
  options: BuildPromptOptions = {}
): string {
  const variant = options.variant ?? 'v1'

  if (variant === 'v2') {
    return buildPromptV2(strategies, options.context)
  }

  // default to v1
  return buildPromptV1(strategies)
}

/**
 * Read prompt_variant from omniown.toml [ai] config.
 * Keeps config reading logic centralized.
 */
export function resolvePromptVariant(
  aiConfig?: Record<string, string>
): PromptVariant {
  const variant = aiConfig?.prompt_variant
  if (variant === 'v1' || variant === 'v2') return variant
  return 'v1'
}
