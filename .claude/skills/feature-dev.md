# feature-dev — 新功能开发

## 触发条件
- 用户说"开发 X 功能"、"实现 X"、"添加 X 端点"、"新增 X 模块"

## 流程

### 1. 需求分析
- 阅读相关现有代码（相关模块 + 数据模型 + API 路由）
- 理解现有架构和数据流
- 输出：设计要点（不写代码）

### 2. 设计确认
- 确定：接口签名 / 数据模型变更 / 文件变更清单 / 影响范围
- 涉及 Rust + TypeScript 双端时，明确两端接口契约
- 获取用户确认后再编码

### 3. 小步编码
- 每完成一个独立单元（≤200 行）停下来验证
- 涉及 DB schema 变更时，先改 `schema.prisma` → `prisma db push` → 再写业务代码
- Rust 端新增功能需同步 `db.rs` 操作

### 4. 自测
- 逐项对照设计要点检查
- 运行 `cargo test` + `npx tsc --noEmit` + `npx vue-tsc --noEmit`

### 5. 提交
- 小原子提交，中文 commit message

## 检查清单
- [ ] API 端点有对应的服务层函数？
- [ ] Rust 端改动有对应的测试？
- [ ] DB schema 变更已同步到 `data-model/schema.md`？
- [ ] 新端点已更新 `api-docs/v1/overview.md`？
