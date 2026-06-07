# architecture — 架构决策

## 触发条件
- 用户说"设计 X 模块"、"选型"、"重构 X"、"这个怎么设计"

## 流程

### 1. 分析现有架构
- 读取 `knowledge/architecture/overview.md`
- 阅读相关模块代码
- 识别约束条件（现有 ADR、技术栈、性能要求）

### 2. 提出方案（至少 2 个）
- 每个方案包含：实现要点、优势、风险、工作量估算
- 涉及跨层变更时，明确每层的影响范围

### 3. ADR 记录
- 决策文档写入 `knowledge/architecture/adr/`，编号递增
- 格式：背景 → 决策 → 实施方案 → 后果

### 4. 更新 overview
- 架构变更后同步更新 `knowledge/architecture/overview.md`

## 架构约束

OmniOwn 特有约束：
- 三层架构不可扁平化（Tauri Shell → Node.js API → Rust CLI）
- Rust CLI 不引入 HTTP 依赖
- 数据库访问双端同步（Schema 变更需 Prisma + rusqlite 双端确认）
- 配置双端读同一文件（TOML），通过 env var 桥接路径
- 前端通过 HTTP 访问 API，不通过 Tauri IPC（保持浏览器兼容性）
