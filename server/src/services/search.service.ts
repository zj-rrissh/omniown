// FTS5 全文搜索 — 8 策略池，LLM 动态选择

import prisma from '../db/client.js'

export interface SearchResult {
  id: number
  filename: string
  storedPath: string
  folderType: string
  category: string
  snippet: string
  rank: number
  updatedAt: string
}

interface Strategy {
  name: string
  description: string
  execute: (params: Record<string, string>) => Promise<SearchResult[]>
}

// 真正的策略列表在模块底部定义（因为要引用下面的 query 函数）

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

interface StrategyCall {
  strategy: string
  params: Record<string, string>
}

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

// --- 策略 1: fulltext — FTS5 全文搜索 ---
async function queryFulltext(term: string): Promise<SearchResult[]> {
  const raw = await prisma.$queryRaw<Array<{
    id: bigint; filename: string; storedPath: string; folderType: string; category: string; snippet: string; rank: number; updatedAt: string
  }>>`
    SELECT
      d.id,
      d.filename,
      d.stored_path AS storedPath,
      d.folder_type AS folderType,
      d.category,
      snippet(documents_fts, 1, '<mark>', '</mark>', '...', 32) AS snippet,
      rank,
      d.updated_at AS updatedAt
    FROM documents_fts
    LEFT JOIN documents d ON d.id = documents_fts.rowid
    WHERE documents_fts MATCH ${term}
    ORDER BY rank
    LIMIT 20
  `
  return raw.map((r) => ({ id: Number(r.id), filename: r.filename, storedPath: r.storedPath, folderType: r.folderType, category: r.category, snippet: r.snippet, rank: r.rank, updatedAt: r.updatedAt }))
}

// --- 策略 2: category — 分类 + domain 匹配 ---
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
      id: true, filename: true, storedPath: true, folderType: true,
      category: true, content: true, updatedAt: true,
    },
    orderBy: { updatedAt: 'desc' },
    take: 20,
  })

  return docs.map((d) => ({
    id: d.id,
    filename: d.filename,
    storedPath: d.storedPath,
    folderType: d.folderType,
    category: d.category,
    snippet: (d.content ?? '').slice(0, 100),
    rank: -1,
    updatedAt: d.updatedAt.toISOString(),
  }))
}

// --- 策略 3: filetype — 文件扩展名 + docType 匹配 ---
async function queryByFileType(ext: string): Promise<SearchResult[]> {
  const docs = await prisma.document.findMany({
    where: {
      OR: [
        { fileExt: ext.replace('.', '') },
        { docType: { contains: ext } },
      ],
    },
    select: {
      id: true, filename: true, storedPath: true, folderType: true,
      category: true, content: true, updatedAt: true,
    },
    orderBy: { updatedAt: 'desc' },
    take: 20,
  })

  return docs.map((d) => ({
    id: d.id,
    filename: d.filename,
    storedPath: d.storedPath,
    folderType: d.folderType,
    category: d.category,
    snippet: (d.content ?? '').slice(0, 100),
    rank: -1,
    updatedAt: d.updatedAt.toISOString(),
  }))
}

// --- 策略 4: summary — FTS5 摘要搜索 ---
async function queryBySummary(term: string): Promise<SearchResult[]> {
  const raw = await prisma.$queryRaw<Array<{
    id: bigint; filename: string; storedPath: string; folderType: string; category: string; snippet: string; rank: number; updatedAt: string
  }>>`
    SELECT
      d.id,
      d.filename,
      d.stored_path AS storedPath,
      d.folder_type AS folderType,
      d.category,
      snippet(documents_fts, 3, '<mark>', '</mark>', '...', 32) AS snippet,
      rank,
      d.updated_at AS updatedAt
    FROM documents_fts
    LEFT JOIN documents d ON d.id = documents_fts.rowid
    WHERE documents_fts MATCH ${term}
    ORDER BY rank
    LIMIT 20
  `
  return raw.map((r) => ({ id: Number(r.id), filename: r.filename, storedPath: r.storedPath, folderType: r.folderType, category: r.category, snippet: r.snippet, rank: r.rank, updatedAt: r.updatedAt }))
}

// --- 策略 5: recent — 按导入时间筛选 ---
async function queryRecent(days: number): Promise<SearchResult[]> {
  const since = new Date()
  since.setDate(since.getDate() - days)

  const docs = await prisma.document.findMany({
    where: {
      importedAt: { gte: since },
    },
    select: {
      id: true, filename: true, storedPath: true, folderType: true,
      category: true, content: true, updatedAt: true,
    },
    orderBy: { updatedAt: 'desc' },
    take: 20,
  })

  return docs.map((d) => ({
    id: d.id,
    filename: d.filename,
    storedPath: d.storedPath,
    folderType: d.folderType,
    category: d.category,
    snippet: (d.content ?? '').slice(0, 100),
    rank: -1,
    updatedAt: d.updatedAt.toISOString(),
  }))
}

// --- 策略 6: privacy — 公开/私密筛选 ---
async function queryByFolderType(folderType: string): Promise<SearchResult[]> {
  const docs = await prisma.document.findMany({
    where: { folderType: folderType },
    select: {
      id: true, filename: true, storedPath: true, folderType: true,
      category: true, content: true, updatedAt: true,
    },
    orderBy: { updatedAt: 'desc' },
    take: 20,
  })

  return docs.map((d) => ({
    id: d.id,
    filename: d.filename,
    storedPath: d.storedPath,
    folderType: d.folderType,
    category: d.category,
    snippet: (d.content ?? '').slice(0, 100),
    rank: -1,
    updatedAt: d.updatedAt.toISOString(),
  }))
}

// --- 策略 7: filename — 文件名模糊匹配 ---
async function queryByFilename(filename: string): Promise<SearchResult[]> {
  const docs = await prisma.document.findMany({
    where: { filename: { contains: filename } },
    select: {
      id: true, filename: true, storedPath: true, folderType: true,
      category: true, content: true, updatedAt: true,
    },
    orderBy: { updatedAt: 'desc' },
    take: 20,
  })

  return docs.map((d) => ({
    id: d.id,
    filename: d.filename,
    storedPath: d.storedPath,
    folderType: d.folderType,
    category: d.category,
    snippet: (d.content ?? '').slice(0, 100),
    rank: -1,
    updatedAt: d.updatedAt.toISOString(),
  }))
}

// --- 策略 8: tag — 标签匹配 ---
async function queryByTag(tag: string): Promise<SearchResult[]> {
  const docs = await prisma.document.findMany({
    where: { tags: { contains: tag } },
    select: {
      id: true, filename: true, storedPath: true, folderType: true,
      category: true, content: true, updatedAt: true,
    },
    orderBy: { updatedAt: 'desc' },
    take: 20,
  })

  return docs.map((d) => ({
    id: d.id,
    filename: d.filename,
    storedPath: d.storedPath,
    folderType: d.folderType,
    category: d.category,
    snippet: (d.content ?? '').slice(0, 100),
    rank: -1,
    updatedAt: d.updatedAt.toISOString(),
  }))
}

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

export async function searchDocuments(query: string): Promise<SearchResult[]> {
  return executeStrategy('fulltext', { query })
}


