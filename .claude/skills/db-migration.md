# db-migration — 数据库迁移

## 触发条件
- 修改 `schema.prisma`
- 用户说"加字段"、"改表结构"、"数据迁移"

## 流程

### 1. 评估影响
- 读取 `knowledge/data-model/schema.md`
- 列出影响范围：Prisma client 类型、rusqlite 操作、API 响应格式、前端渲染
- 是否需要数据迁移脚本？

### 2. 修改 schema.prisma
- 先改 `server/prisma/schema.prisma`
- 运行 `npx prisma db push --skip-generate`（SQLite 不支持 `migrate`）
- 运行 `npx prisma generate` 更新 client 类型

### 3. 同步 Rust 端
- 新增字段需在 `db.rs` 中对应的 INSERT/UPDATE 语句添加
- 新增字段需在 struct 定义中添加（如有）
- 注意字段名映射（Prisma: camelCase → DB: snake_case）

### 4. 更新文档
- `knowledge/data-model/schema.md` 更新字段表
- 如有数据迁移步骤，记录在 `lessons-learned/`

### 5. 测试验证
- `cargo test` 全部通过
- `npx tsc --noEmit` 类型检查通过

## 安全检查
- 生产数据迁移前备份数据库文件
- 新增字段有合理的默认值
- 删除字段前确认无代码引用
