// ============================================================
// FTS5 全文搜索服务
// ============================================================
//
// 设计原则：
//   不是一条 SQL 搜所有东西，而是维护一个 SQL 策略池。
//   每条策略对应一种搜索意图（搜标题、搜分类、搜时间范围等）。
//   由 LLM 在 ai.search.ts 中分析用户意图，动态选择策略。
//
// 策略池：
//   strategy     字段                适用场景
//   ─────────── ────────────────── ──────────────────
//   fulltext    filename+content    默认：全文搜关键词
//   category    category+domain    "代码相关的文档"
//   filetype    fileExt+docType    "PDF文件"
//   summary     summary            摘要匹配
//   recent      createdAt          最近导入的文件
//   privacy     folderType         "私有文档"
//   filename    filename           搜文件名
//
// ============================================================

import prisma from '../db/client.js'

export interface SearchResult {
  id: number
  filename: string
  snippet: string
  rank: number
}

// ============================================================
// 策略注册表
// ============================================================
//
// 每个条目是一个查询策略，包含：
// - name: 策略标识（LLM 输出时引用）
// - description: 策略说明（给 LLM 看的）
// - execute: 执行函数

interface Strategy {
  name: string
  description: string
  execute: (params: Record<string, string>) => Promise<SearchResult[]>
}

// 真正的策略列表在模块底部定义（因为要引用下面的 query 函数）

// ============================================================
// 主入口：执行指定的搜索策略
// ============================================================

/**
 * 根据 LLM 选择的策略和参数，执行搜索。
 *
 * 调用方（ai.search.ts）用法：
 *   executeStrategy("fulltext", { query: "TypeScript" })
 *   executeStrategy("category", { category: "code" })
 *   executeStrategy("recent", { days: "7" })
 */
export async function executeStrategy(
  strategyName: string,
  params: Record<string, string>
): Promise<SearchResult[]> {
  const strategy = STRATEGIES.find((s) => s.name === strategyName)
  if (!strategy) {
    throw new Error(`未知搜索策略: ${strategyName}`)
  }
  return strategy.execute(params)
}

/**
 * 返回所有可用策略的名称和描述，供 LLM 选择。
 */
export function getAvailableStrategies(): Array<{ name: string; description: string }> {
  return STRATEGIES.map((s) => ({ name: s.name, description: s.description }))
}

// ============================================================
// 批量执行策略 + 合并结果
// ============================================================

interface StrategyCall {
  strategy: string
  params: Record<string, string>
}

/**
 * 并行执行多个搜索策略，合并去重后按 rank 排序返回。
 *
 * 使用场景：
 *   用户说 "我上周的代码文件" → 时间 + 分类，两个意图
 *   LLM 返回 [{ strategy: "recent", params: { days: "7" } },
 *            { strategy: "category", params: { keyword: "code" } }]
 *
 * 合并规则：
 *   1. 所有策略并行执行（Promise.all）
 *   2. 按 id 去重：同一个文档出现在多个策略的结果中 → 保留 rank 更好的那条
 *   3. 按 rank 升序排列（rank 越小越匹配）
 *   4. 每个单独策略失败不影响其他策略（isolated error handling）
 */
export async function executeStrategies(
  calls: StrategyCall[]
): Promise<SearchResult[]> {
  if (calls.length === 0) return []

  // 并行执行所有策略
  // Promise.allSettled 不会因为一个失败而中止所有
  const settled = await Promise.allSettled(
    calls.map((c) => executeStrategy(c.strategy, c.params))
  )

  // 收集所有成功的结果
  const allResults: SearchResult[] = []
  for (const result of settled) {
    if (result.status === 'fulfilled') {
      allResults.push(...result.value)
    }
    // 失败的策略静默跳过，不阻断其他结果
  }

  // 去重 + 排序：
  // 同一个 id 出现在多个策略结果中时，保留 rank 更小的（更匹配）
  const deduped = new Map<number, SearchResult>()
  for (const r of allResults) {
    const existing = deduped.get(r.id)
    if (!existing || r.rank < existing.rank) {
      deduped.set(r.id, r)
    }
  }

  // 按 rank 升序排列，截取前 20 条
  return Array.from(deduped.values())
    .sort((a, b) => a.rank - b.rank)
    .slice(0, 20)
}

// ============================================================
// 底层查询函数（每个函数一种 SQL 查询）
// ============================================================

// --- 策略 1：全文搜索（默认） ---
// 适用：用户输入关键词，如 "TypeScript教程"、"rust async"
// FTS5 MATCH 在 filename 和 content 两列上同时搜索
async function queryFulltext(term: string): Promise<SearchResult[]> {
  const raw = await prisma.$queryRaw<Array<{
    id: bigint; filename: string; snippet: string; rank: number
  }>>`
    SELECT
      d.id,
      d.filename,
      snippet(documents_fts, 1, '<mark>', '</mark>', '...', 32) AS snippet,
      rank
    FROM documents_fts
    LEFT JOIN documents d ON d.id = documents_fts.rowid
    WHERE documents_fts MATCH ${term}
    ORDER BY rank
    LIMIT 20
  `
  return raw.map((r) => ({ id: Number(r.id), filename: r.filename, snippet: r.snippet, rank: r.rank }))
}

// --- 策略 2：按分类搜索 ---
// 适用："代码相关的文档"、"日记类文件"、"合同文件"
// 不走 FTS5，直接查 documents 表，用 LIKE 匹配 category 和 domain
async function queryByCategory(keyword: string): Promise<SearchResult[]> {
  const docs = await prisma.document.findMany({
    where: {
      // Prisma OR 条件：category 或 domain 包含关键词
      OR: [
        { category: { contains: keyword } },
        { domain: { contains: keyword } },
      ],
    },
    select: {
      id: true, filename: true, content: true,
      category: true, updatedAt: true,
    },
    orderBy: { updatedAt: 'desc' },
    take: 20,
  })

  return docs.map((d) => ({
    id: d.id,
    filename: d.filename,
    // 没有 FTS5 snippet，手动截取前 100 字
    snippet: (d.content ?? '').slice(0, 100),
    // 分类搜索没有 rank 概念，统一给 -1
    rank: -1,
  }))
}

// --- 策略 3：按文件类型搜索 ---
// 适用："PDF文件"、"Markdown文档"
async function queryByFileType(ext: string): Promise<SearchResult[]> {
  const docs = await prisma.document.findMany({
    where: {
      OR: [
        { fileExt: ext.replace('.', '') },
        { docType: { contains: ext } },
      ],
    },
    select: {
      id: true, filename: true, content: true, updatedAt: true,
    },
    orderBy: { updatedAt: 'desc' },
    take: 20,
  })

  return docs.map((d) => ({
    id: d.id,
    filename: d.filename,
    snippet: (d.content ?? '').slice(0, 100),
    rank: -1,
  }))
}

// --- 策略 4：摘要搜索 ---
// 适用：查询中提及"关于..."、"内容是..."等描述性语言
// FTS5 的 content 列默认包含 text columns 的所有数据
// 这里在 summary 列上执行 MATCH
async function queryBySummary(term: string): Promise<SearchResult[]> {
  const raw = await prisma.$queryRaw<Array<{
    id: bigint; filename: string; snippet: string; rank: number
  }>>`
    SELECT
      d.id,
      d.filename,
      snippet(documents_fts, 3, '<mark>', '</mark>', '...', 32) AS snippet,
      rank
    FROM documents_fts
    LEFT JOIN documents d ON d.id = documents_fts.rowid
    WHERE documents_fts MATCH ${term}
    ORDER BY rank
    LIMIT 20
  `
  return raw.map((r) => ({ id: Number(r.id), filename: r.filename, snippet: r.snippet, rank: r.rank }))
}

// --- 策略 5：最近导入 ---
// 适用："最近的文件"、"几天前的文档"、"上周的文件"
async function queryRecent(days: number): Promise<SearchResult[]> {
  const since = new Date()
  since.setDate(since.getDate() - days)

  const docs = await prisma.document.findMany({
    where: {
      importedAt: { gte: since },
    },
    select: {
      id: true, filename: true, content: true, importedAt: true,
    },
    orderBy: { importedAt: 'desc' },
    take: 20,
  })

  return docs.map((d) => ({
    id: d.id,
    filename: d.filename,
    snippet: (d.content ?? '').slice(0, 100),
    rank: -1,
  }))
}

// --- 策略 6：按公开/隐私 ---
// 适用："私有文档"、"公开文件"
async function queryByFolderType(folderType: string): Promise<SearchResult[]> {
  const docs = await prisma.document.findMany({
    where: { folderType: folderType },
    select: {
      id: true, filename: true, content: true, updatedAt: true,
    },
    orderBy: { updatedAt: 'desc' },
    take: 20,
  })

  return docs.map((d) => ({
    id: d.id,
    filename: d.filename,
    snippet: (d.content ?? '').slice(0, 100),
    rank: -1,
  }))
}

// --- 策略 7：按文件名搜索 ---
// 适用："名叫 xxx 的文件"、"文件名包含 test 的"
async function queryByFilename(filename: string): Promise<SearchResult[]> {
  const docs = await prisma.document.findMany({
    where: { filename: { contains: filename } },
    select: {
      id: true, filename: true, content: true, updatedAt: true,
    },
    orderBy: { updatedAt: 'desc' },
    take: 20,
  })

  return docs.map((d) => ({
    id: d.id,
    filename: d.filename,
    snippet: (d.content ?? '').slice(0, 100),
    rank: -1,
  }))
}

// --- 策略 8：标签搜索 ---
// 适用："带某个标签的文件"
async function queryByTag(tag: string): Promise<SearchResult[]> {
  const docs = await prisma.document.findMany({
    where: { tags: { contains: tag } },
    select: {
      id: true, filename: true, content: true, updatedAt: true,
    },
    orderBy: { updatedAt: 'desc' },
    take: 20,
  })

  return docs.map((d) => ({
    id: d.id,
    filename: d.filename,
    snippet: (d.content ?? '').slice(0, 100),
    rank: -1,
  }))
}

// ============================================================
// 策略注册表（放在底层查询函数定义之后）
// ============================================================

const STRATEGIES: Strategy[] = [
  { name: 'fulltext',  description: '全文搜索（文件名+内容）',        execute: (p) => queryFulltext(p.query ?? '') },
  { name: 'category',  description: '按分类搜索（笔记/代码/财务等）',  execute: (p) => queryByCategory(p.keyword ?? '') },
  { name: 'filetype',  description: '按文件类型搜索（PDF/Markdown等）', execute: (p) => queryByFileType(p.ext ?? '') },
  { name: 'summary',   description: '按摘要内容搜索',                  execute: (p) => queryBySummary(p.query ?? '') },
  { name: 'recent',    description: '按时间范围搜索最近的文件',         execute: (p) => queryRecent(Number(p.days) || 7) },
  { name: 'privacy',   description: '按公开/隐私状态搜索',              execute: (p) => queryByFolderType(p.folderType ?? '') },
  { name: 'filename',  description: '按文件名搜索',                    execute: (p) => queryByFilename(p.filename ?? '') },
  { name: 'tag',       description: '按标签搜索',                      execute: (p) => queryByTag(p.tag ?? '') },
]

// ============================================================
// 导出一个便捷的默认策略执行器
// ============================================================

/**
 * 默认搜索（当没有 AI 分析时使用）。
 * 先尝试 FTS5 全文搜索，结果太少再补充分类搜索。
 */
export async function searchDocuments(query: string): Promise<SearchResult[]> {
  return executeStrategy('fulltext', { query })
}

// ============================================================
// 学习笔记
// ============================================================
//
// 策略模式的优点：
//   1. 新搜索方式 = 新 query 函数 + 注册表加一行
//   2. 每个 query 函数独立，不会互相干扰
//   3. LLM 只需要输出策略名，不用自己拼 SQL
//
// 为什么不把所有字段塞进一个巨复杂的 SQL？
//   1. SQL 复杂度指数级增长
//   2. 查询优化器难以选择正确索引
//   3. 难以调试哪一个条件导致了错误结果
//
// 下一步（ai.search.ts 修改）：
//   LLM 的 system prompt 中列出所有可用策略名和参数
//   LLM 输出 { strategy: "fulltext", params: { query: "TypeScript" } }
//   调用 executeStrategy(strategy, params)
