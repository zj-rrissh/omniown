# 数据模型

## 核心表：documents

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | Int (PK, auto) | 主键 |
| `filename` | String | 文件名 |
| `originalPath` | String? | 原始绝对路径 |
| `storedPath` | String (unique) | 业务唯一键，library 内相对路径 |
| `fileExt` | String? | 文件扩展名（小写） |
| `fileSize` | Int? | 文件大小（字节） |
| `fileHash` | String | SHA256 内容哈希（去重依据） |
| `folderType` | String | public / private |
| `category` | String | 分类标签 |
| `domain` | String | 领域标签 |
| `docType` | String | 文档类型 |
| `content` | String? | 提取的纯文本 |
| `summary` | String? | AI 摘要（预留） |
| `tags` | String? | 逗号分隔标签 |
| `privacyScore` | Float? | 隐私评分（0-1） |
| `riskLevel` | String | 风险等级：low/medium/high |
| `processingStatus` | String | pending / indexed / failed |
| `embeddingStatus` | String | 已废弃（pending） |
| `summaryStatus` | String | 已废弃（skipped） |
| `createdAt` | DateTime | 创建时间 |
| `updatedAt` | DateTime | 更新时间 |
| `importedAt` | DateTime | 导入时间 |

## 虚拟表：documents_fts（FTS5）

```
documents_fts(filename, content, tags, summary)
content='documents', content_rowid='id'
```

## 触发器

| 触发器 | 事件 | 行为 |
|------|------|------|
| `documents_ai` | INSERT | 插入 FTS 索引 |
| `documents_ad` | DELETE | 从 FTS 索引删除 |
| `documents_au` | UPDATE | 先删旧索引，再插新索引 |

## 实体关系

```
documents ────1:1──→ documents_fts (通过 id=rowid 同步)
```

当前为单表设计，无外键关联。
