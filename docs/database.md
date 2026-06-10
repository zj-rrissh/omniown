# 数据库文档

OmniOwn 使用 **SQLite** 作为元数据和全文索引存储，通过 **Prisma ORM v5** 驱动，辅以 **FTS5 raw SQL** 实现全文搜索。

---

## 数据库文件位置

| 运行模式 | 数据库路径 |
|:---|:---|
| Tauri 桌面端 | `{app_data_dir}/omniown.db`（由 `DATABASE_URL` 环境变量指定） |
| Node.js 独立运行 | `server/prisma/dev.db` |

启动时通过 `prisma db push --skip-generate` 自动建表（幂等）。

---

## Schema 管理

使用 Prisma Schema 声明式定义数据结构：

- **Schema 文件**：`server/prisma/schema.prisma`
- **客户端生成**：`prisma generate`（`server build` 时自动执行）
- **表同步**：`prisma db push --skip-generate`（首次启动时自动执行，幂等）
- **FTS5**：Prisma 不支持 FTS5 虚拟表，通过 `server/src/db/setup-fts.ts` 手动创建

---

## Documents 表

主文档表，存储每份文件的元数据和全文内容。

```prisma
model Document {
  id               Int      @id @default(autoincrement())
  filename         String
  originalPath     String?                // 原始路径
  storedPath       String   @unique       // library 中的存储路径
  fileExt          String?                // 文件扩展名
  fileSize         Int?                   // 文件大小（字节）
  fileHash         String                 // SHA256 内容哈希（去重判断）
  folderType       String   @default("public")  // public / private
  category         String   @default("misc")    // 分类
  domain           String   @default("unknown") // 来源域
  docType          String   @default("unknown") // 文档类型
  content          String?                // 文件全文
  summary          String?                // 摘要（预留）
  tags             String?                // 标签（逗号分隔，预留）
  privacyScore     Float?   @default(0)   // 隐私分数 0-1
  riskLevel        String   @default("low")     // low / medium / high
  processingStatus String   @default("pending") // pending → indexed → failed
  embeddingStatus  String   @default("pending") // ⚠️ 已废弃，保留向后兼容
  summaryStatus    String   @default("skipped") // ⚠️ 已废弃，保留向后兼容
  createdAt        DateTime @default(now())
  updatedAt        DateTime @updatedAt
  importedAt       DateTime @default(now())

  @@map("documents")
}
```

**关键字段：**

- `storedPath` — 文件在 `library/` 下的存储路径，业务唯一键
- `fileHash` — 提取正文的 SHA256 哈希，用于 `upsert` 时检测内容变更
- `folderType` — `public` / `private`，对应 `library/` 下的子目录
- `category` — 分类标签，由 processor 模块基于关键词自动分配
- `processingStatus` — `pending` → `indexed`（导入成功）/ `failed`（失败）
- `embeddingStatus` / `summaryStatus` — 已废弃，保留字段以兼容旧数据

**索引：**

- `idx_documents_hash` (fileHash)
- `idx_documents_folderType` (folderType)
- `idx_documents_category` (category)
- `idx_documents_processingStatus` (processingStatus)
- `idx_documents_embeddingStatus` (embeddingStatus) — 废弃
- `idx_documents_updatedAt` (updatedAt)

---

## documents_fts（FTS5 全文检索）

由 `server/src/db/setup-fts.ts` 在启动时创建，Prisma 不管理此表。

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
    filename,
    content,
    tags,
    summary,
    content='documents',
    content_rowid='id'
);
```

通过三个触发器与 `documents` 表保持同步：

- `documents_ai` — INSERT 时写入 FTS 索引
- `documents_ad` — DELETE 时从 FTS 索引删除
- `documents_au` — UPDATE 时删除旧索引并写入新索引

---

## document_embeddings（已废弃）

> ⚠️ **v0.1.0 起废弃。** 不再有代码写入或读取此表。保留在 Schema 中以保持向后兼容。

向量 embedding 存储表（历史参考）：

```prisma
model DocumentEmbedding {
  documentId Int      @map("document_id")
  modelName  String   @map("model_name")
  dim        Int
  vector     String   // Base64 编码
  createdAt  DateTime @default(now()) @map("created_at")
  updatedAt  DateTime @updatedAt @map("updated_at")

  @@id([documentId, modelName])
  @@map("document_embeddings")
}
```

复合主键 `(documentId, modelName)` 允许同一文档保存多个模型的 embedding。

---

## 数据库初始化流程

```
server 启动 (index.ts)
  ↓
1. prisma db push --skip-generate (建表/同步 Schema，幂等)
  ↓
2. initFts5() (创建 documents_fts 虚拟表 + 触发器，幂等)
  ↓
3. 挂载 API 路由
  ↓
4. listen(3001)
```

---

## 开发操作

```bash
# 重新同步 Schema（添加字段/索引后）
cd server && npx prisma db push

# 可视化浏览
cd server && npx prisma studio

# 重新生成 Prisma Client（修改 schema.prisma 后）
cd server && npx prisma generate
```
