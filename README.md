# Claude Usage Monitor

macOS 選單列應用程式，即時監控 Claude Pro / Max 訂閱的配額使用量。

## 功能

- **選單列圖示**：常駐於 macOS 選單列，不佔用 Dock 空間
- **Pro 方案**：以圓形進度環顯示 5-Hour 與 7-Day 的使用百分比
- **Max 方案**：Session 與 Weekly 圓形進度環 + Sonnet Only 橫條進度列
- **配額重置自動刷新**：偵測重置時間，到點後 5 秒自動取得最新數據，不再顯示過期的 100%
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

1. 前往 [Releases](https://github.com/HaoJi666/claude-usage-monitor/releases) 下載對應版本：
   - **Apple Silicon (M1/M2/M3/M4)**：`Claude.Usage.Monitor_*_aarch64.dmg`
   - **Intel Mac**：`Claude.Usage.Monitor_*_x64.dmg`
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

- **查看用量**：點擊選單列圖示展開主視窗
- **重新整理**：點擊右上角的重新整理圖示手動更新數據
- **收起視窗**：點擊視窗外部任意處或再次點擊選單列圖示

### Pro 方案顯示

| 圓環 | 說明 |
|------|------|
| 5-Hour | 滾動 5 小時內的累計用量 |
| 7-Day | 滾動 7 天內的累計用量 |

### Max 方案顯示

| 元件 | 說明 |
|------|------|
| Session（圓環） | 當前 session 用量 |
| Weekly（圓環） | 7 天全模型累計用量 |
| Sonnet only（橫條） | 7 天 Sonnet 模型專屬用量 |

### 圓環顏色含義

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
# Apple Silicon
npm run tauri build

# Intel (需先安裝 cross-compilation target)
rustup target add x86_64-apple-darwin
npm run tauri build -- --target x86_64-apple-darwin
```

輸出在 `src-tauri/target/[target]/release/bundle/dmg/`。

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

### 方案識別

1. `/overage_spend_limit` 的 `seat_tier` 欄位（最可靠）
2. `/subscription_details` 的 `seat_tier_quantities` 物件
3. `/api/usage` 回應中 `seven_day_sonnet` 欄位是否存在（Max 專屬）
4. 任何 API 回應中的 `plan_type` 字串

## 隱私說明

- 所有資料均在本機處理，不會傳送至任何第三方伺服器
- Session cookies 由 WKWebView 管理，應用程式本身不儲存密碼
- 僅讀取用量相關 API，不存取對話內容

## 更新紀錄

### v0.2.0
- Max 方案完整支援：Session / Weekly 圓環 + Sonnet Only 橫條
- 方案識別改善：多層判斷邏輯，從 billing API 優先取得
- 修復配額重置後不自動刷新的問題
- 刷新時先預取帳號/訂閱資料

### v0.1.0
- 初始版本
