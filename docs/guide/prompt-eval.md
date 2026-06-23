# Prompt Eval 评估方案

> 用于量化比较不同 System Prompt 变体（v1 / v2）对 AI 搜索策略选择的影响，
> 从而指导 prompt 迭代优化。

---

## 1. 背景

AI 搜索（`ai.service.ts`）通过 LLM 分析用户自然语言查询，从 8 个策略中选择最匹配的组合执行。
System Prompt 的质量直接影响策略选择的**准确性**和**稳定性**。

目前支持 `v1`（原始）和 `v2`（增强—Few-shot + 文档库上下文 + fallback 兜底）两个变体，
通过 `omniown.toml` 的 `[ai].prompt_variant` 切换。需要一个量化方法来评估效果。

---

## 2. 评估方案

### 方案：批处理 Eval 脚本

创建一个 `server/scripts/eval-prompts.ts`，定义一组测试用例，分别用 v1 / v2 运行，
收集追踪数据，输出对比报告。

```
        ┌─────────────┐
测试用例  │ 查询 + 期望策略 │
 集合     └──────┬──────┘
                 ▼
         ┌──────────────────┐
         │ 分别用 v1 / v2 跑  │  → /api/search?q=xxx&ai=true&variant=v1
         │  全部测试用例       │  → /api/search?q=xxx&ai=true&variant=v2
         └──────────────────┘
                 ▼
         ┌──────────────────┐
         │ 对比 2 个维度的    │
         │ 指标，输出报告      │
         └──────────────────┘
```

---

## 3. 评估维度

| 维度 | 数据来源 | 说明 |
|------|----------|------|
| **策略选择准确率** | `trace.selectedStrategies` | LLM 选择了哪些策略？与期望策略匹配度如何？ |
| **策略选择稳定性** | 同一查询跑 3 次，检查一致性 | LLM temperature=0.1 时应基本稳定 |
| **合并结果数** | `trace.mergedResultCount` | 最终返回了多少条结果（去重后 top 20） |
| **LLM Token 消耗** | 原始响应长度 | 影响成本和延迟 |

### 准确率评分规则

```typescript
function scoreStrategies(selected: string[], expected: string[]): number {
  const hits = expected.filter(s => selected.includes(s)).length
  const extras = selected.filter(s => !expected.includes(s)).length
  return Math.max(0, (hits - extras * 0.5) / expected.length)
}
```

---

## 4. 测试用例

### 4.1 Few-shot 场景（v2 明确给出过，v1 没有）

| # | 查询 | 期望策略 | 说明 |
|---|------|----------|------|
| 1 | `我上周写的机器学习笔记` | `[recent, category, fulltext]` | 时间 + 主题 + 内容 |
| 2 | `所有的PDF文件` | `[filetype]` | 纯文件类型 |
| 3 | `关于Docker的代码教程` | `[category, fulltext]` | 分类 + 内容 |
| 4 | `私密的财务数据` | `[privacy, category]` | 隐私 + 分类 |
| 5 | `最近3天的日记` | `[recent, fulltext]` | 时间 + 内容 |

### 4.2 边界情况

| # | 查询 | 期望策略 | 说明 |
|---|------|----------|------|
| 6 | `上周的东西` | `[recent]` | 非常模糊，仅时间 |
| 7 | `Python` | `[fulltext]` | 仅关键词，无修饰 |
| 8 | `公开的文件` | `[privacy]` | 仅状态 |
| 9 | `给我找点有意思的` | `[fulltext]` | 无效意图 → 兜底 |

### 4.3 多意图组合

| # | 查询 | 期望策略 | 说明 |
|---|------|----------|------|
| 10 | `最近7天的技术文档PDF` | `[recent, filetype, category]` | 时间 + 类型 + 分类 |
| 11 | `上周写的公开的Java代码` | `[recent, privacy, category, fulltext]` | 四维度组合 |
| 12 | `私密日记中的财务记录` | `[privacy, category, fulltext]` | 隐私 + 分类 + 内容 |

---

## 5. 输出报告

### 终端输出

```
╔════════════════════════════════════════════════════════╗
║  Prompt Eval Report                                    ║
╠════════════════════════════════════════════════════════╣
║  Variant: v1  vs  v2                                   ║
║  Test cases: 12                                        ║
╠════════════════════════════════════════════════════════╣
║ #  Query              v1_strategies     v2_strategies  ║
║ 1  上周机器学习笔记    [recent,fulltext]  [recent,cat..] ✓║
║ 2  所有PDF文件         [fulltext,file..]  [filetype]    ✓║
║ ...                                                    ║
╠════════════════════════════════════════════════════════╣
║  Accuracy:     41.7% (v1)  vs  83.3% (v2)              ║
║  Avg strategies: 1.6 (v1)  vs  2.1 (v2)                ║
║  Avg results:   9.2 (v1)  vs  12.8 (v2)                ║
╚════════════════════════════════════════════════════════╝
```

### JSON 输出（供自动化消费）

```json
{
  "variant": "v2",
  "timestamp": "2026-06-23T12:00:00Z",
  "cases": [
    { "query": "我上周写的机器学习笔记", "accuracy": 1.0, "strategies": [...], "resultCount": 15 }
  ],
  "summary": {
    "accuracy": 0.833,
    "avgStrategies": 2.1,
    "avgResults": 12.8,
    "stability": 0.83
  }
}
```

---

## 6. 实现步骤

### 步骤 1：扩展 API 支持传入 variant

`server/src/api/search.ts` 增加 `variant` 查询参数：

```
GET /api/search?q=xxx&ai=true&variant=v2
```

**涉及文件：**
- `server/src/services/ai.service.ts` — `aiSearchWithTrace` 增加 `variant` 参数
- `server/src/api/search.ts` — 解析 `req.query.variant` 传给 ai service

### 步骤 2：创建评估脚本

**新建 `server/scripts/eval-prompts.ts`**，功能：

- 遍历测试用例
- 对每个用例调用 `/api/search?q=xxx&ai=true&variant=xxx`
- 解析 `trace` 数据
- 计算准确率、稳定性（可选重复 3 次）
- 输出终端报告 + JSON 文件

**运行：**
```bash
npx tsx scripts/eval-prompts.ts

# 仅跑某个 variant
npx tsx scripts/eval-prompts.ts --variant v2

# 输出 JSON
npx tsx scripts/eval-prompts.ts --json ./eval-results.json

# 稳定性测试（重复 3 次）
npx tsx scripts/eval-prompts.ts --repeat 3
```

### 步骤 3：集成到开发流程

```
修改 prompt → npx tsx scripts/eval-prompts.ts
  ↓
检查准确率变化
  ↓
如果提升 → 提交
如果下降 → 回退或继续调整
```

---

## 7. 涉及文件

| 文件 | 操作 | 说明 |
|------|------|------|
| `server/src/services/ai.service.ts` | 修改 | `aiSearchWithTrace` 增加可选 `variant` 参数 |
| `server/src/api/search.ts` | 修改 | 解析 `req.query.variant` |
| `server/scripts/eval-prompts.ts` | **新建** | 批处理评估脚本 |
| `docs/guide/prompt-eval.md` | **本文** | 评估方案文档 |

---

## 8. 后续扩展

- **稳定版存档**：每次评估自动保存报告到 `docs/eval-reports/`，形成历史趋势
- **模糊匹配**：期望策略支持同义策略组（如 `fulltext` ≈ `summary`）
- **真实用户查询回放**：从搜索历史中取真实查询做测试集
- **CI 集成**：PR 中自动跑 eval，阻止准确率下降的 prompt 变更合入
