---
description: 關閉所有測試時開啟的 Recap app 和開發伺服器
allowed-tools: Bash(pkill*), Bash(ps*), Bash(kill*)
---

## 任務：關閉測試相關進程

請執行以下步驟關閉所有測試時開啟的應用程式：

### 1. 列出目前運行的相關進程

```bash
ps aux | grep -E "(recap|tauri|vite)" | grep -v grep
```

### 2. 關閉進程

依序關閉：

1. **Recap 應用程式** (測試時啟動的 debug build)
   ```bash
   pkill -f "target/debug/recap"
   ```

2. **Tauri 開發伺服器**
   ```bash
   pkill -f "cargo-tauri"
   ```

3. **Vite 開發伺服器**
   ```bash
   pkill -f "node.*vite"
   ```

4. **esbuild 服務**
   ```bash
   pkill -f "esbuild.*service"
   ```

### 3. 確認已關閉

```bash
ps aux | grep -E "(recap|tauri|vite)" | grep -v grep || echo "所有相關進程已關閉"
```

### 注意事項
- 此命令會關閉所有 Recap 相關的開發進程
- 如果有正在進行的測試，請確保已完成再執行
- 不會影響生產環境的應用程式
