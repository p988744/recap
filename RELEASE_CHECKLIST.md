# v2.1.0 Release Checklist

## 版本進程

```
beta.1 → beta.N → rc.1 → rc.N → v2.1.0 (stable)
         ↑                ↑
      修正 bugs       最終驗證
```

---

## Beta → RC 檢查項目

### 功能完整性

- [ ] Projects 頁面功能正常
- [ ] ThisWeek 頁面 manual items 顯示正確
- [ ] Gantt chart 顯示 manual items
- [ ] Antigravity badge 顯示正確
- [ ] 同步時間戳跨重啟保持

### 核心功能迴歸測試

- [ ] Git commits 捕獲與顯示
- [ ] Claude Code sessions 同步
- [ ] Work items CRUD 操作
- [ ] Reports 生成與匯出
- [ ] Settings 頁面所有設定可正常儲存
- [ ] Tempo 匯出功能
- [ ] GitLab 整合

### 自動化測試

- [ ] `cargo test --workspace` 全部通過
- [ ] `npm test` 全部通過
- [ ] TypeScript 無編譯錯誤 (`npm run typecheck`)

### 跨平台驗證

- [ ] macOS (Apple Silicon) - 安裝、啟動、基本功能
- [ ] macOS (Intel) - 安裝、啟動、基本功能
- [ ] Windows - 安裝、啟動、基本功能
- [ ] Linux - 安裝、啟動、基本功能

### 效能檢查

- [ ] App 啟動時間 < 3 秒
- [ ] 大量資料 (1000+ work items) 不卡頓
- [ ] 記憶體使用合理 (< 500MB)

### 已知 Issues

- [ ] 所有 P0/P1 bugs 已修復
- [ ] P2 bugs 已評估，非阻擋性的可延後

---

## RC → Release 檢查項目

### 穩定性確認

- [ ] RC 版本使用 3+ 天無重大問題
- [ ] 無新增 crash 或 data loss 回報
- [ ] 效能無明顯退化

### 文件更新

- [ ] CHANGELOG.md 已更新
- [ ] README.md 版本資訊正確
- [ ] 使用者文件已同步

### 發布準備

- [ ] 版本號更新 (移除 -rc 後綴)
  - [ ] `web/package.json`
  - [ ] `web/src-tauri/Cargo.toml`
  - [ ] `web/src-tauri/tauri.conf.json`
- [ ] Git tag 建立 (`v2.1.0`)
- [ ] GitHub Release draft 準備
- [ ] Release notes 撰寫完成

### CI/CD 驗證

- [ ] Release workflow 成功執行
- [ ] 所有平台 artifacts 產生
- [ ] Artifacts 可正常下載安裝

### 最終確認

- [ ] 從 Release 下載安裝測試
- [ ] 基本 smoke test 通過
- [ ] 團隊 sign-off

---

## 版本號更新指令

### Beta → RC

```bash
# 更新版本號
npm pkg set version=2.1.0-rc.1
sed -i '' 's/version = "2.1.0-beta.[0-9]*"/version = "2.1.0-rc.1"/' src-tauri/Cargo.toml
sed -i '' 's/"version": "2.1.0-beta.[0-9]*"/"version": "2.1.0-rc.1"/' src-tauri/tauri.conf.json

# Commit & Tag
git add -A && git commit -m "chore: Bump version to 2.1.0-rc.1"
git tag v2.1.0-rc.1
git push && git push --tags
```

### RC → Release

```bash
# 更新版本號
npm pkg set version=2.1.0
sed -i '' 's/version = "2.1.0-rc.[0-9]*"/version = "2.1.0"/' src-tauri/Cargo.toml
sed -i '' 's/"version": "2.1.0-rc.[0-9]*"/"version": "2.1.0"/' src-tauri/tauri.conf.json

# Commit & Tag
git add -A && git commit -m "chore: Release v2.1.0"
git tag v2.1.0
git push && git push --tags
```

---

## 問題追蹤

| Issue | 優先級 | 狀態 | 備註 |
|-------|--------|------|------|
| | | | |

---

## Sign-off

| 角色 | 姓名 | 日期 | 簽核 |
|------|------|------|------|
| 開發 | | | ☐ |
| QA | | | ☐ |
| PM | | | ☐ |

---

> 最後更新: 2026-02-04
