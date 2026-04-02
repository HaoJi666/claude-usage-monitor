# 產品需求文件 (PRD)

## 產品名稱

Claude Usage Monitor

## 版本

v0.1.0

## 最後更新

2026-04-02

---

## 1. 產品概述

### 1.1 背景

Claude Pro 與 Max 訂閱方案對每 5 小時與每 7 天的用量設有配額限制。官方介面（claude.ai/settings/usage）需要手動開啟瀏覽器才能查看，對於高頻使用者而言不夠直覺。

### 1.2 目標

提供一個輕量的 macOS 選單列工具，讓使用者無需開啟瀏覽器即可隨時掌握 Claude 配額使用狀況，在配額耗盡前收到視覺警示。

### 1.3 目標使用者

- Claude Pro / Max 訂閱使用者
- 以 Claude 作為主要工作工具的開發者、研究者或創作者
- 需要在多個任務間管理 Claude 用量的重度使用者

---

## 2. 功能需求

### 2.1 核心功能

#### F1：選單列常駐圖示
- 應用程式以選單列模式執行，不顯示於 Dock
- 圖示旁顯示目前 5 小時用量百分比
- 點擊圖示切換主視窗顯示 / 隱藏
- 右鍵選單提供「Exit App」選項

#### F2：配額儀表板
- 以 SVG 圓形進度環顯示 5 小時用量（左）與 7 天用量（右）
- 環內顯示百分比數字（取整數）
- 環下方顯示配額重置時間
- 配色依用量動態調整：綠（< 50%）、黃（50–79%）、紅（≥ 80%）

#### F3：超額用量區塊 (Extra Usage)
- 顯示條件：帳號有啟用 Extra Usage 功能
- 呈現方式：標題列（Extra Usage 標題 + On/Off 徽章）、花費金額與重置日期、水平進度條、月度上限與預付餘額
- 數據來源：`/api/usage`（花費、限額）、`/prepaid/credits`（餘額、自動儲值狀態）、`/subscription_details`（重置日期）

#### F4：自動刷新
- 應用程式啟動後 5 秒開始第一次刷新
- 背景依設定間隔（預設 5 分鐘）定時導航 session webview 至 `/settings/usage` 觸發 API 請求
- 支援間隔選項：2 分鐘 / 5 分鐘 / 10 分鐘

#### F5：手動刷新
- 主視窗標頭提供刷新按鈕
- 點擊時立即觸發 `trigger_refresh` 指令
- 刷新期間顯示旋轉動畫，最長等待 8 秒後自動停止
- 資料更新後顯示最後更新時間（Footer）

#### F6：設定視窗
- 點擊標頭齒輪圖示開啟獨立設定視窗（label: `settings`）
- 若視窗已開啟則 focus 而非重複建立
- 包含以下區塊：
  - **Account**：顯示登入狀態、Email、方案類型；提供登入 / 登出按鈕
  - **How it works**：說明資料取得方式、外部連結
  - **Preferences**：刷新間隔設定、儲存按鈕

#### F7：登入流程
- 點擊「Open Claude.ai Login」開啟 session webview（480×720）至 `claude.ai/login`
- 支援 Google OAuth 登入（透過 Tauri popup window 中繼 postMessage）
- 登入成功後自動導航至 `/settings/usage` 取得資料
- 登入狀態以 `login-status-changed` 事件同步至主視窗

#### F8：自動隱藏
- 主視窗失去焦點時自動隱藏（Rust `on_window_event(Focused(false))`）
- 與 `ActivationPolicy::Accessory` 相容（JavaScript 的 focus 事件在此模式不可靠）

#### F9：主視窗自動縮放
- 主視窗依內容高度自動調整（ResizeObserver + `getCurrentWindow().setSize()`）
- 寬度固定 360px，高度隨內容變化

### 2.2 資料持久化
- 使用 SQLite 儲存用量紀錄與設定（`refresh_interval_secs`）
- 資料庫位於 macOS App Data 目錄（`~/Library/Application Support/com.claudeusagemonitor/`）
- 啟動時讀取快取資料立即顯示，背景刷新後再更新

---

## 3. 非功能需求

### 3.1 隱私與安全
- 所有資料在本機處理，不經由第三方伺服器
- 不儲存使用者密碼，Session 由 WKWebView cookie 管理
- CSP 設為 null（允許 claude.ai 內容正確載入）

### 3.2 效能
- 選單列視窗開啟延遲 < 100ms
- 刷新資料取得時間 < 5 秒（正常網路環境）

### 3.3 相容性
- macOS 10.15 (Catalina) 或以上
- Apple Silicon (arm64) 與 Intel (x86_64) 皆支援

---

## 4. 技術架構

### 4.1 前端
- React 19 + TypeScript（Vite 建置）
- Tailwind CSS v4（utility-first 樣式）
- 視窗路由：`getCurrentWindow().label === "settings"` 決定渲染 `App` 或 `SettingsPage`

### 4.2 後端 (Rust / Tauri 2.0)
- `AppState`：持有 SQLite 連線、最新用量快取、登入狀態、session email
- Session webview（外部，載入 claude.ai）注入 fetch 攔截器 + URL 監控器
- OAuth popup window 中繼 `window.opener.postMessage`

### 4.3 資料流
```
claude.ai API → fetch 攔截器 (JS) → cm_api_data (Rust IPC) → AppState 快取
                                                              ↓
                                                    usage-updated (Tauri event)
                                                              ↓
                                                       主視窗 UI 更新
```

### 4.4 Tauri 指令清單

| 指令 | 呼叫方 | 說明 |
|------|--------|------|
| `get_usage` | 主視窗 | 讀取最新用量快取 |
| `get_login_status` | 主視窗 / 設定視窗 | 讀取登入狀態 |
| `open_login_window` | 設定視窗 | 顯示 session webview 並導航至登入頁 |
| `close_login_window` | 設定視窗 | 隱藏 session webview |
| `open_settings_window` | 主視窗 | 開啟設定視窗 |
| `trigger_refresh` | 主視窗 | 導航 session 至 `/settings/usage` |
| `logout` | 主視窗 / 設定視窗 | 清除登入狀態並呼叫 POST /api/auth/logout |
| `get_settings` | 設定視窗 | 讀取偏好設定 |
| `save_settings` | 設定視窗 | 儲存偏好設定 |
| `cm_login_check` | Session JS | URL 變化時同步登入狀態 |
| `cm_api_data` | Session JS | 傳遞攔截到的 API 回應 |
| `cm_open_popup` | Session JS | 建立 OAuth popup 視窗 |
| `cm_popup_navigated` | Popup JS | OAuth redirect 完成偵測 |
| `cm_oauth_message` | Popup JS | 中繼 postMessage 至 session 視窗 |

---

## 5. 已知限制

- 依賴 claude.ai 內部 API 格式，官方如變更欄位名稱需更新解析邏輯
- macOS 限定，不支援 Windows / Linux
- 需要網路連線才能取得最新數據

---

## 6. 未來規劃

- [ ] 用量通知：接近配額上限時發送系統通知
- [ ] 用量歷史圖表：以折線圖顯示近期用量趨勢
- [ ] 多帳號支援
- [ ] 自動啟動：系統開機時自動在背景執行
- [ ] 快捷鍵：全域快捷鍵呼叫視窗
