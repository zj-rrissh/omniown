// FTS5 全文索引初始化 — Prisma 不支持 FTS5，手动创建虚拟表 + 触发器

import prisma from './client.js'

export async function initFts5(): Promise<void> {
  try {
    console.log('[db] 初始化 FTS5 全文索引...')

    await prisma.$executeRawUnsafe(`
      CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
        filename, content, tags, summary,
        content='documents', content_rowid='id'
      )
    `)

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
    if (msg.includes('already exists')) return
    console.error('[db] FTS5 初始化失败:', msg)
  }
}

if (process.argv[1]?.includes('setup-fts')) {
  initFts5().then(() => prisma.$disconnect())
}
