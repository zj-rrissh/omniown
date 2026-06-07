---
date: 2026-06-07
tags: [config, desktop, paths]
context: 修复桌面打包后 Node.js 端配置读取失败导致 watch 监听错误路径
confidence: high
---

# 打包后配置路径桥接

## 问题

Tauri 将配置写入 OS 用户数据目录（`app_config_dir()`），Node.js 端从 `__dirname` 推算 exe 根目录读取。两条路径不一致导致：
1. `loadConfig()` 返回 `{}`
2. `--library` 不传给 Rust CLI
3. watch 监听默认空目录而非用户配置的 library

## 解决方案

1. Tauri `spawn_sidecar` 通过 `OMNIOWN_CONFIG_PATH` 环境变量传入配置文件绝对路径
2. Node.js `config/index.ts` 优先读取 `process.env.OMNIOWN_CONFIG_PATH`
3. 重启路径同步补全 env vars（DATABASE_URL + OMNIOWN_CONFIG_PATH + current_dir）

## 注意事项

- 环境变量传递是最简洁的跨进程配置桥接方式
- 重启逻辑需要完整复制初始 spawn 的所有 env/current_dir 参数
