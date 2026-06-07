# security-audit — 安全检查

## 触发条件
- 用户说"安全检查"、"security audit"
- 提交前、发布前

## 检查清单

### Secret 管理
- [ ] 无硬编码密钥/密码/token
- [ ] `server/.env` 和 `omniown.toml` 在 .gitignore 中
- [ ] API key 在 IPC 中脱敏返回（前 4 位 + `***`）
- [ ] 日志中无敏感数据

### 输入验证
- [ ] API 请求参数有类型校验？
- [ ] 搜索查询字符串有长度限制？
- [ ] 文件路径无目录遍历风险？（`../` 转义）
- [ ] 文件大小有限制？

### 认证与授权
- [ ] API 端点是否需要鉴权？（当前为本地应用，localhost only）
- [ ] CORS 白名单正确？（CSP 中 `connect-src 'self' http://127.0.0.1:3001`）
- [ ] MCP Server 仅本地 stdio 通信（无网络暴露）

### 依赖安全
- [ ] `cargo audit` 无已知漏洞
- [ ] `npm audit` 无高危漏洞
- [ ] 新增依赖 license 兼容（MIT）？

### 数据安全
- [ ] 数据库文件权限合理？
- [ ] 错误消息不泄露路径/内部 ID？
- [ ] 日志文件有轮转/大小限制？

## 当前安全边界

OmniOwn 是本地桌面应用，安全模型以本地信任为基础：
- API 仅监听 127.0.0.1:3001（不对外暴露）
- MCP Server 通过 stdio 通信（无网络端口）
- 数据库存储在 app_data_dir（用户专有目录）
- 无用户认证系统（本地单用户）
