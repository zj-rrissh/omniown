/**
 * 策略输出验证 — zod JSON Schema 验证
 *
 * 替代 ai.service.ts 中的裸 JSON.parse(...) as StrategyCall[]
 * 在 LLM 返回格式不合法时早期拦截，给出明确错误信息。
 */

import { z } from 'zod'

// ---- Schema 定义 ----

/** 单个策略调用的 zod schema */
export const StrategyCallSchema = z.object({
  strategy: z.enum([
    'fulltext',
    'category',
    'filetype',
    'summary',
    'recent',
    'privacy',
    'filename',
    'tag',
  ]),
  params: z.record(z.string(), z.string()),
})

/** 策略数组的 zod schema（至少 1 个策略） */
export const StrategyResponseSchema = z.array(StrategyCallSchema).min(1)

// ---- 类型推导 ----

/** 从 zod schema 推导的已验证策略调用类型 */
export type ValidatedStrategyCall = z.infer<typeof StrategyCallSchema>

// ---- 验证函数 ----

/**
 * 验证 LLM 返回的策略数组。
 *
 * @param raw - 已 JSON.parse 的原始对象
 * @returns 验证通过的 StrategyCall[]
 * @throws 如果格式不合法，抛出含详细错误信息的 Error
 */
export function validateStrategies(raw: unknown): ValidatedStrategyCall[] {
  const result = StrategyResponseSchema.safeParse(raw)

  if (!result.success) {
    const issues = result.error.issues
      .map((i) => `  - ${i.path.join('.') || '(root)'}: ${i.message}`)
      .join('\n')
    throw new Error(`策略输出格式验证失败:\n${issues}\n\n原始输出: ${JSON.stringify(raw)}`)
  }

  return result.data
}
