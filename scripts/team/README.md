# 團隊協作腳本

這些腳本用於簡化團隊協作流程，確保分支管理規範。

## 腳本列表

| 腳本 | 說明 | 使用時機 |
|------|------|----------|
| `sync-develop.sh` | 同步 develop 分支 | 每天開始工作前 |
| `pre-pr-check.sh` | PR 提交前檢查 | 提交 PR 前 |
| `check-boundaries.sh` | 檢查職責邊界 | 確認沒有越界修改 |
| `create-pr.sh` | 建立 PR（含檢查） | 準備提交 PR 時 |
| `status.sh` | 顯示團隊狀態 | 了解整體進度 |

## 使用方式

### 每日工作流程

```bash
# 1. 開始工作前，同步 develop
./scripts/team/sync-develop.sh

# 2. 進行開發...

# 3. 完成後，檢查 PR 條件
./scripts/team/pre-pr-check.sh

# 4. 建立 PR
./scripts/team/create-pr.sh
```

### 查看團隊狀態

```bash
./scripts/team/status.sh
```

### 檢查職責邊界

```bash
./scripts/team/check-boundaries.sh
```

## 快速設定

建議將以下 alias 加入 `~/.bashrc` 或 `~/.zshrc`：

```bash
# Recap 團隊腳本
alias recap-sync="./scripts/team/sync-develop.sh"
alias recap-check="./scripts/team/pre-pr-check.sh"
alias recap-pr="./scripts/team/create-pr.sh"
alias recap-status="./scripts/team/status.sh"
```

## 注意事項

- 這些腳本應在 worktree 根目錄下執行
- 需要安裝 `gh` (GitHub CLI) 才能使用 PR 相關功能
- 腳本會自動判斷你的角色（根據分支名稱）
