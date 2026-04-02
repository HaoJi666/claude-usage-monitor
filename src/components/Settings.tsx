import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";

interface LoginStatus {
  is_logged_in: boolean;
  email: string | null;
  plan_type: string | null;
}

interface AppSettings {
  refresh_interval_secs: number;
}

interface SettingsProps {
  onClose: () => void;
}

export default function Settings({ onClose }: SettingsProps) {
  const [loginStatus, setLoginStatus] = useState<LoginStatus>({
    is_logged_in: false,
    email: null,
    plan_type: null,
  });
  const [settings, setSettings] = useState<AppSettings>({ refresh_interval_secs: 300 });
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<{ type: "error" | "success"; text: string } | null>(null);

  useEffect(() => {
    loadData();

    // Listen for login state changes from the session webview
    const unlisten = listen<boolean>("login-status-changed", (event) => {
      if (event.payload) {
        // Just logged in — reload status
        loadData();
      } else {
        setLoginStatus({ is_logged_in: false, email: null, plan_type: null });
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  async function loadData() {
    try {
      const [status, stgs] = await Promise.all([
        invoke<LoginStatus>("get_login_status"),
        invoke<AppSettings>("get_settings"),
      ]);
      setLoginStatus(status);
      setSettings(stgs);
    } catch (e) {
      showMessage("error", String(e));
    }
  }

  async function handleLogin() {
    try {
      await invoke("open_login_window");
    } catch (e) {
      showMessage("error", String(e));
    }
  }

  async function handleHideLoginWindow() {
    try {
      await invoke("close_login_window");
    } catch (e) {
      showMessage("error", String(e));
    }
  }

  async function handleLogout() {
    try {
      await invoke("logout");
      setLoginStatus({ is_logged_in: false, email: null, plan_type: null });
      showMessage("success", "Logged out.");
    } catch (e) {
      showMessage("error", String(e));
    }
  }

  async function handleSaveSettings() {
    setSaving(true);
    try {
      await invoke("save_settings", { settings });
      showMessage("success", "Settings saved!");
    } catch (e) {
      showMessage("error", String(e));
    } finally {
      setSaving(false);
    }
  }

  function showMessage(type: "error" | "success", text: string) {
    setMessage({ type, text });
    setTimeout(() => setMessage(null), 3000);
  }

  const planLabel = loginStatus.plan_type
    ? ` · ${loginStatus.plan_type}`
    : "";

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between px-5 py-4 border-b border-black/10 dark:border-white/10">
        <h2 className="text-base font-semibold text-gray-900 dark:text-gray-100">Settings</h2>
        <button
          onClick={onClose}
          className="w-7 h-7 flex items-center justify-center rounded-full bg-black/5 dark:bg-white/5 hover:bg-black/10 dark:hover:bg-white/10 text-gray-600 dark:text-gray-400 transition-colors"
        >
          ✕
        </button>
      </div>

      <div className="flex-1 overflow-y-auto px-5 py-4 space-y-6">

        {/* ── Account section ── */}
        <section>
          <h3 className="text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-3">
            Account
          </h3>

          {loginStatus.is_logged_in ? (
            /* Logged-in state */
            <div className="space-y-3">
              <div className="flex items-center gap-3 p-3 rounded-xl bg-emerald-50 dark:bg-emerald-900/20 border border-emerald-200 dark:border-emerald-800">
                <div className="w-8 h-8 rounded-full bg-emerald-500 flex items-center justify-center text-white text-sm font-bold shrink-0">
                  ✓
                </div>
                <div className="min-w-0">
                  <p className="text-sm font-medium text-emerald-800 dark:text-emerald-300">
                    Connected to Claude.ai
                  </p>
                  {loginStatus.email && (
                    <p className="text-xs text-emerald-600 dark:text-emerald-400 truncate">
                      {loginStatus.email}{planLabel}
                    </p>
                  )}
                  {!loginStatus.email && loginStatus.plan_type && (
                    <p className="text-xs text-emerald-600 dark:text-emerald-400">
                      {loginStatus.plan_type}
                    </p>
                  )}
                </div>
              </div>
              <button
                onClick={handleLogout}
                className="w-full py-2 text-sm font-medium rounded-lg bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 hover:bg-red-100 dark:hover:bg-red-900/40 border border-red-200 dark:border-red-800 transition-colors"
              >
                Logout
              </button>
            </div>
          ) : (
            /* Logged-out state */
            <div className="space-y-3">
              <div className="p-3 rounded-xl bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800">
                <p className="text-sm font-medium text-amber-800 dark:text-amber-300 mb-1">
                  Not connected
                </p>
                <p className="text-xs text-amber-600 dark:text-amber-400">
                  Log in with your Claude.ai account to monitor Pro/Max plan usage.
                </p>
              </div>

              <button
                onClick={handleLogin}
                className="w-full py-2.5 text-sm font-medium rounded-lg bg-[#d97706] hover:bg-[#b45309] text-white transition-colors flex items-center justify-center gap-2"
              >
                <span>Open Claude.ai Login</span>
                <span className="text-xs opacity-80">↗</span>
              </button>

              <p className="text-xs text-gray-400 dark:text-gray-500 text-center">
                A browser window will open. Log in, then come back here.
              </p>

              <button
                onClick={handleHideLoginWindow}
                className="w-full py-2 text-sm text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 transition-colors"
              >
                Hide login window
              </button>
            </div>
          )}
        </section>

        {/* ── How to get usage data section ── */}
        <section>
          <h3 className="text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-3">
            How it works
          </h3>
          <div className="p-3 rounded-xl bg-black/5 dark:bg-white/5 space-y-2 text-xs text-gray-600 dark:text-gray-400 leading-relaxed">
            <p>
              This app monitors your <strong className="text-gray-800 dark:text-gray-200">Claude Pro / Max</strong> subscription usage by reading from claude.ai.
            </p>
            <ol className="list-decimal list-inside space-y-1 pl-1">
              <li>Click <em>Open Claude.ai Login</em> above</li>
              <li>Log in with your Claude account</li>
              <li>Close the login window — usage is fetched automatically</li>
            </ol>
            <div className="pt-1 space-y-1.5">
              <button
                onClick={() => openUrl("https://claude.ai/settings/usage")}
                className="flex items-center gap-1.5 text-blue-500 hover:text-blue-600 dark:text-blue-400 transition-colors"
              >
                <span>📊</span>
                <span className="underline underline-offset-2">View usage on Claude.ai</span>
              </button>
              <button
                onClick={() => openUrl("https://claude.ai/upgrade")}
                className="flex items-center gap-1.5 text-blue-500 hover:text-blue-600 dark:text-blue-400 transition-colors"
              >
                <span>⭐</span>
                <span className="underline underline-offset-2">Upgrade to Claude Pro / Max</span>
              </button>
            </div>
          </div>
        </section>

        {/* ── Preferences section ── */}
        <section>
          <h3 className="text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-3">
            Preferences
          </h3>
          <div className="space-y-3">
            <div className="flex items-center justify-between p-3 rounded-xl bg-black/5 dark:bg-white/5">
              <div>
                <p className="text-sm font-medium text-gray-800 dark:text-gray-200">Refresh interval</p>
                <p className="text-xs text-gray-500 dark:text-gray-400">How often to fetch usage data</p>
              </div>
              <select
                value={settings.refresh_interval_secs}
                onChange={(e) => setSettings({ refresh_interval_secs: Number(e.target.value) })}
                className="text-sm bg-black/5 dark:bg-white/10 border border-black/10 dark:border-white/10 rounded-lg px-2 py-1 text-gray-800 dark:text-gray-200 focus:outline-none"
              >
                <option value={120}>2 min</option>
                <option value={300}>5 min</option>
                <option value={600}>10 min</option>
              </select>
            </div>
            <button
              onClick={handleSaveSettings}
              disabled={saving}
              className="w-full py-2 text-sm font-medium rounded-lg bg-gray-700 hover:bg-gray-800 dark:bg-gray-600 dark:hover:bg-gray-500 disabled:opacity-50 text-white transition-colors"
            >
              {saving ? "Saving…" : "Save Settings"}
            </button>
          </div>
        </section>
      </div>

      {/* Status messages */}
      {message && (
        <div
          className={`mx-5 mb-4 px-3 py-2 rounded-lg text-sm ${
            message.type === "error"
              ? "bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400"
              : "bg-green-50 dark:bg-green-900/20 text-green-600 dark:text-green-400"
          }`}
        >
          {message.text}
        </div>
      )}
    </div>
  );
}
