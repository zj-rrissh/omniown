# release — 发布管理

## 触发条件
- 用户说"发布"、"release"、"推送 release"

## 流程

### 1. 发布前检查
- [ ] `cargo test` 全部通过（172 个）
- [ ] `cargo fmt -- --check` 无格式问题
- [ ] `cargo clippy -- -D warnings` 零警告
- [ ] `cd server && npx tsc --noEmit` 通过
- [ ] `cd ui && npx vue-tsc --noEmit` 通过
- [ ] CHANGELOG.md 已更新

### 2. 版本号更新
- `Cargo.toml`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`
- `server/package.json`、`ui/package.json`
- 使用语义化版本：`v<MAJOR>.<MINOR>.<PATCH>`

### 3. 打标签
```bash
git tag -a v0.1.0 -m "v0.1.0: 发布说明"
git push origin v0.1.0
```

### 4. 创建 Release
```bash
gh release create v0.1.0 --title "v0.1.0" --notes-file CHANGELOG.md
```

### 5. 验证 CI
- 等待 Release workflow 完成
- 验证三平台产物：`.dmg` / `.exe` / `.AppImage`

## 注意事项
- Tag 推送触发 Release CI → 构建 sidecar → 构建 Tauri 安装包
- Release 创建后才会触发自动构建
- 如需更新 Release Notes：直接在 GitHub 页面编辑，不要重新打 tag
