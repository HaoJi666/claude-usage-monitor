# Claude Usage Monitor

macOS 選單列應用程式，即時監控 Claude Pro / Max 訂閱的配額使用量。

## 功能

- **選單列圖示**：常駐於 macOS 選單列，不佔用 Dock 空間
- **配額儀表板**：以圓形進度環顯示 5 小時與 7 天的用量百分比
- **超額用量追蹤**：顯示 Extra Usage 的花費金額、月度上限與預付餘額
- **自動刷新**：背景定時取得最新用量，可設定間隔（2 / 5 / 10 分鐘）
- **手動刷新**：點擊重新整理按鈕可立即取得最新數據
- **自動縮放**：主視窗依內容高度自動調整，不留多餘空白
- **設定視窗**：點擊齒輪圖示開啟獨立設定頁面（帳號管理、偏好設定）
- **自動隱藏**：點擊視窗外部時自動收起，不干擾工作流程

## 系統需求

- macOS 10.15 (Catalina) 或以上
- Claude Pro 或 Max 訂閱帳號
- 需登入 [claude.ai](https://claude.ai) 授權資料存取

## 安裝

1. 下載最新版本的 `.dmg` 安裝檔（見 Releases）
2. 開啟 `.dmg`，將應用程式拖曳至「應用程式」資料夾
3. 啟動 Claude Usage Monitor

> 首次啟動時 macOS 可能提示「無法驗證開發者」，請至「系統設定 → 隱私權與安全性」點擊「仍要打開」。

## 使用方式

### 首次設定

1. 點擊選單列的 Claude 圖示開啟監控視窗
2. 點擊右上角的 **齒輪圖示** 開啟設定視窗
3. 在「Account」區塊點擊 **Open Claude.ai Login**
4. 於彈出的登入視窗輸入 Claude 帳號並完成驗證
5. 登入成功後設定視窗會顯示連線狀態，主視窗自動顯示用量

### 日常使用

- **查看用量**：點擊選單列圖示展開主視窗，可看到 5 小時與 7 天的使用百分比
- **重新整理**：點擊右上角的重新整理圖示手動更新數據
- **收起視窗**：點擊視窗外部任意處或再次點擊選單列圖示

### 圓形進度環顏色含義

| 顏色 | 用量 |
|------|------|
| 綠色 | < 50% |
| 黃色 | 50–79% |
| 紅色 | ≥ 80% |

### 設定選項

| 選項 | 說明 |
|------|------|
| Refresh Interval | 背景自動刷新的間隔時間（2 / 5 / 10 分鐘） |
| Logout | 登出 Claude 帳號並清除本機資料 |

## 開發

### 環境需求

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) 1.77+
- [Tauri CLI v2](https://tauri.app/start/prerequisites/)

### 本機開發

```bash
# 安裝前端相依套件
npm install

# 啟動開發模式（熱重載）
npm run tauri dev
```

### 打包

```bash
npm run tauri build
```

輸出在 `src-tauri/target/release/bundle/`。

## 技術架構

| 層級 | 技術 |
|------|------|
| 框架 | [Tauri 2.0](https://tauri.app/) |
| 前端 | React 19 + TypeScript + Tailwind CSS v4 |
| 後端 | Rust（資料解析、IPC、視窗管理） |
| 資料庫 | SQLite（rusqlite，儲存設定與用量紀錄） |
| 資料來源 | claude.ai 內部 API（透過嵌入式 session webview 攔截 fetch） |

### 資料取得原理

應用程式在背景維持一個隱藏的 session webview（載入 claude.ai）。透過注入 JavaScript fetch 攔截器，捕捉 `/api/usage`、`/prepaid/credits` 和 `/subscription_details` 等 API 回應，再透過 Tauri IPC 傳至 Rust 端解析與儲存，最後以 `usage-updated` 事件推送至主視窗 UI。

## 隱私說明

- 所有資料均在本機處理，不會傳送至任何第三方伺服器
- Session cookies 由 WKWebView 管理，應用程式本身不儲存密碼
- 僅讀取用量相關 API，不存取對話內容
