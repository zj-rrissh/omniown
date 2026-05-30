// ============================================================
// 数据库初始化脚本
// ============================================================
//
// 用 Prisma Schema 创建 documents 表后，还需要用原始 SQL
// 创建 FTS5 虚拟表和同步触发器。Prisma 不支持 FTS5。
//
// 运行方式：
//   cd server && npx tsx src/db/setup-fks.ts
//
// ============================================================

import prisma from './client.js'

async function setupFts5() {
  // --- 创建 FTS5 虚拟表 ---
  //
  // content='documents' 表示使用 documents 表作为外部内容表，
  // 不需要 INSERT 数据到 FTS5 表，只需在 documents 上 INSERT/UPDATE/DELETE。
  // 同步由以下 triggers 自动完成。

  console.log('创建 FTS5 虚拟表...')
  await prisma.$executeRaw`
    CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
      filename,
      content,
      tags,
      summary,
      content='documents',
      content_rowid='id'
    )
  `

  // --- 创建 triggers ---
  //
  // 这三个 trigger 在 documents 表增/删/改时自动同步 FTS5 索引。

  // INSERT：新建文档 → 同步到 FTS5
  console.log('创建 trigger: documents_ai')
  await prisma.$executeRaw`
    CREATE TRIGGER IF NOT EXISTS documents_ai AFTER INSERT ON documents BEGIN
      INSERT INTO documents_fts(rowid, filename, content, tags, summary)
      VALUES (new.id, new.filename, new.content, new.tags, new.summary)
    END
  `

  // DELETE：删除文档 → 从 FTS5 移除
  console.log('创建 trigger: documents_ad')
  await prisma.$executeRaw`
    CREATE TRIGGER IF NOT EXISTS documents_ad AFTER DELETE ON documents BEGIN
      INSERT INTO documents_fts(documents_fts, rowid, filename, content, tags, summary)
      VALUES('delete', old.id, old.filename, old.content, old.tags, old.summary)
    END
  `

  // UPDATE：更新文档 → 先删旧索引，再插新索引
  console.log('创建 trigger: documents_au')
  await prisma.$executeRaw`
    CREATE TRIGGER IF NOT EXISTS documents_au AFTER UPDATE ON documents BEGIN
      INSERT INTO documents_fts(documents_fts, rowid, filename, content, tags, summary)
      VALUES('delete', old.id, old.filename, old.content, old.tags, old.summary);
      INSERT INTO documents_fts(rowid, filename, content, tags, summary)
      VALUES (new.id, new.filename, new.content, new.tags, new.summary)
    END
  `

  console.log('FTS5 索引创建完成')
  await prisma.$disconnect()
}

export async function initFts5(): Promise<void> {
  try {
    console.log('[db] 初始化 FTS5 全文索引...')

    // CREATE VIRTUAL TABLE
    await prisma.$executeRawUnsafe(`
      CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
        filename, content, tags, summary,
        content='documents', content_rowid='id'
      )
    `)

    // Triggers — 用 $executeRawUnsafe 避免模板字符串的分号问题
    await prisma.$executeRawUnsafe(`
      CREATE TRIGGER IF NOT EXISTS documents_ai AFTER INSERT ON documents
      BEGIN
        INSERT INTO documents_fts(rowid, filename, content, tags, summary)
        VALUES (new.id, new.filename, new.content, new.tags, new.summary);
      END
    `)

    await prisma.$executeRawUnsafe(`
      CREATE TRIGGER IF NOT EXISTS documents_ad AFTER DELETE ON documents
      BEGIN
        INSERT INTO documents_fts(documents_fts, rowid, filename, content, tags, summary)
        VALUES('delete', old.id, old.filename, old.content, old.tags, old.summary);
      END
    `)

    await prisma.$executeRawUnsafe(`
      CREATE TRIGGER IF NOT EXISTS documents_au AFTER UPDATE ON documents
      BEGIN
        INSERT INTO documents_fts(documents_fts, rowid, filename, content, tags, summary)
        VALUES('delete', old.id, old.filename, old.content, old.tags, old.summary);
        INSERT INTO documents_fts(rowid, filename, content, tags, summary)
        VALUES (new.id, new.filename, new.content, new.tags, new.summary);
      END
    `)

    console.log('[db] FTS5 索引就绪')
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err)
    // 如果已经存在，忽略错误
    if (msg.includes('already exists')) return
    console.error('[db] FTS5 初始化失败:', msg)
  }
}

// 也支持直接运行此脚本
if (process.argv[1]?.includes('setup-fts')) {
  initFts5().then(() => prisma.$disconnect())
}
