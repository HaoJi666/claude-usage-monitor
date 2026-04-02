import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import { useUsage, ExtraUsage, PeriodUsage } from "./hooks/useUsage";
import Settings from "./components/Settings";

export default function App() {
  const { usage, loading, error, isLoggedIn, refetch } = useUsage();
  const [showSettings, setShowSettings] = useState(false);
  // Focus-loss auto-hide is handled on the Rust side (on_window_event).

  async function handleLogout() {
    try { await invoke("logout"); } catch (_) {}
  }

  return (
    <div className="w-[360px] h-screen bg-white dark:bg-[#1c1c1e] overflow-hidden rounded-2xl shadow-2xl select-none flex flex-col">
      {/* Header */}
      <div className="flex-shrink-0 flex items-center justify-between px-4 py-3 border-b border-black/10 dark:border-white/10">
        <div className="flex items-center gap-2">
          <div className="w-5 h-5 bg-gradient-to-br from-orange-400 to-red-500 rounded-md flex items-center justify-center">
            <span className="text-white text-[10px] font-bold">C</span>
          </div>
          <div className="flex items-baseline gap-1.5">
            <span className="text-sm font-semibold text-gray-900 dark:text-gray-100">Claude Usage</span>
            {!showSettings && usage?.plan_type && (
              <span className="text-[10px] font-medium text-orange-500 dark:text-orange-400 uppercase tracking-wider">
                {usage.plan_type}
              </span>
            )}
          </div>
        </div>
        <div className="flex items-center gap-1.5">
          {!showSettings && (
            <button
              onClick={refetch}
              disabled={loading}
              title="Refresh"
              className="w-6 h-6 flex items-center justify-center rounded-full bg-black/5 dark:bg-white/5 hover:bg-black/10 dark:hover:bg-white/10 text-gray-500 dark:text-gray-400 transition-colors disabled:opacity-40"
            >
              <RefreshIcon spinning={loading} />
            </button>
          )}
          <button
            onClick={() => setShowSettings(!showSettings)}
            title={showSettings ? "Close settings" : "Settings"}
            className={`w-6 h-6 flex items-center justify-center rounded-full transition-colors ${
              showSettings
                ? "bg-orange-500 text-white"
                : "bg-black/5 dark:bg-white/5 hover:bg-black/10 dark:hover:bg-white/10 text-gray-500 dark:text-gray-400"
            }`}
          >
            <SettingsIcon />
          </button>
          {!showSettings && isLoggedIn && (
            <button onClick={handleLogout} title="Sign out"
              className="w-6 h-6 flex items-center justify-center rounded-full bg-black/5 dark:bg-white/5 hover:bg-red-100 dark:hover:bg-red-900/30 text-gray-500 dark:text-gray-400 hover:text-red-500 dark:hover:text-red-400 transition-colors"
            >
              <LogoutIcon />
            </button>
          )}
        </div>
      </div>

      {showSettings ? (
        /* Settings panel fills remaining space */
        <div className="flex-1 min-h-0 overflow-hidden">
          <Settings onClose={() => setShowSettings(false)} />
        </div>
      ) : (
        <>
          {/* Content */}
          <div className="flex-1 min-h-0 overflow-y-auto px-4 py-3">
            {error ? (
              <ErrorState error={error} onRetry={refetch} />
            ) : !isLoggedIn ? (
              <NotLoggedInPrompt />
            ) : loading && !usage ? (
              <LoadingState />
            ) : usage ? (
              <div className="space-y-3">
                {/* 5h + 7d circles */}
                <div className="flex items-center justify-around">
                  <CircleGauge label="5-Hour" period={usage.five_hour} />
                  <div className="w-px h-16 bg-black/10 dark:bg-white/10" />
                  <CircleGauge label="7-Day" period={usage.seven_day} />
                </div>

                {/* Extra usage */}
                {usage.extra_usage && (
                  <ExtraUsageSection extra={usage.extra_usage} />
                )}
              </div>
            ) : (
              <LoadingState />
            )}
          </div>

          {/* Footer */}
          <div className="flex-shrink-0 px-4 pb-2.5">
            <p className="text-[10px] text-gray-400 dark:text-gray-500 text-center">
              {usage?.fetched_at ? `Updated ${formatTime(usage.fetched_at)}` : "Claude Pro / Max Usage Monitor"}
            </p>
          </div>
        </>
      )}
    </div>
  );
}

// ── Circular gauge ──────────────────────────────────────────────────────────

function CircleGauge({ label, period }: { label: string; period: PeriodUsage }) {
  const pct = Math.min(period.utilization, 100);
  const r = 34;
  const circ = 2 * Math.PI * r;
  const offset = circ - (pct / 100) * circ;
  const color = pct >= 80 ? "#ef4444" : pct >= 50 ? "#eab308" : "#10b981";

  const resetsDate = (() => {
    try {
      return new Date(period.resets_at).toLocaleDateString(undefined, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
    } catch {
      return period.resets_at;
    }
  })();

  return (
    <div className="flex flex-col items-center gap-1">
      <svg width="88" height="88" viewBox="0 0 88 88">
        <circle cx="44" cy="44" r={r} fill="none" stroke="currentColor"
          className="text-black/10 dark:text-white/10" strokeWidth="7" />
        <circle cx="44" cy="44" r={r} fill="none" stroke={color} strokeWidth="7"
          strokeDasharray={circ} strokeDashoffset={offset}
          strokeLinecap="round" transform="rotate(-90 44 44)"
          style={{ transition: "stroke-dashoffset 0.7s ease" }} />
        <text x="44" y="40" textAnchor="middle" fill="currentColor"
          className="text-gray-800 dark:text-gray-100"
          style={{ fontSize: "15px", fontWeight: 700, fill: "currentColor" }}>
          {Math.round(pct)}%
        </text>
        <text x="44" y="54" textAnchor="middle"
          style={{ fontSize: "9px", fill: "#9ca3af" }}>
          {label}
        </text>
      </svg>
      <p className="text-[9px] text-gray-400 dark:text-gray-500 text-center leading-tight">
        Resets {resetsDate}
      </p>
    </div>
  );
}

// ── Extra usage section (compact) ───────────────────────────────────────────

function ExtraUsageSection({ extra }: { extra: ExtraUsage }) {
  const pct = Math.min(extra.percent_used, 100);
  const barColor = pct >= 80 ? "bg-red-500" : pct >= 50 ? "bg-yellow-500" : "bg-blue-500";

  const resetsLabel = extra.resets_at ? (() => {
    try {
      return new Date(extra.resets_at).toLocaleDateString(undefined, { month: "short", day: "numeric" });
    } catch { return extra.resets_at; }
  })() : null;

  return (
    <div className="border-t border-black/10 dark:border-white/10 pt-2.5 space-y-1.5">
      {/* Title + badge + % */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-1.5">
          <span className="text-[11px] font-semibold text-gray-700 dark:text-gray-300">Extra Usage</span>
          <span className={`text-[9px] font-medium px-1 py-0.5 rounded-full ${extra.enabled ? "bg-blue-100 text-blue-600 dark:bg-blue-900/40 dark:text-blue-400" : "bg-gray-100 text-gray-500 dark:bg-white/10 dark:text-gray-500"}`}>
            {extra.enabled ? "On" : "Off"}
          </span>
        </div>
        <span className="text-[11px] font-semibold text-gray-700 dark:text-gray-200">{Math.round(pct)}%</span>
      </div>

      {/* Thin progress bar */}
      <div className="h-1.5 bg-black/10 dark:bg-white/10 rounded-full overflow-hidden">
        <div className={`h-full rounded-full transition-all duration-700 ${barColor}`} style={{ width: `${pct}%` }} />
      </div>

      {/* One-line summary */}
      <div className="flex justify-between text-[10px] text-gray-500 dark:text-gray-400">
        <span>
          ${extra.spent.toFixed(2)} / ${extra.limit.toFixed(0)}
          {resetsLabel ? ` · resets ${resetsLabel}` : ""}
        </span>
        <span>
          ${extra.balance.toFixed(2)} bal
          {extra.auto_reload && <span className="ml-1 text-green-500 dark:text-green-400">↻</span>}
        </span>
      </div>
    </div>
  );
}

// ── Utility components ──────────────────────────────────────────────────────

function NotLoggedInPrompt() {
  async function openLogin() {
    try { await invoke("open_login_window"); } catch (_) {}
  }
  return (
    <div className="flex flex-col items-center gap-3 text-center py-4 px-3">
      <div className="w-11 h-11 bg-orange-100 dark:bg-orange-900/30 rounded-full flex items-center justify-center">
        <span className="text-2xl">☁</span>
      </div>
      <div>
        <p className="text-sm font-semibold text-gray-800 dark:text-gray-200 mb-0.5">
          Connect your Claude account
        </p>
        <p className="text-xs text-gray-500 dark:text-gray-400">
          Log in to monitor your Pro / Max usage.
        </p>
      </div>
      <button onClick={openLogin}
        className="px-5 py-2 text-sm font-medium bg-[#d97706] hover:bg-[#b45309] text-white rounded-xl transition-colors">
        Open Claude.ai Login
      </button>
    </div>
  );
}

function LoadingState() {
  return (
    <div className="flex flex-col items-center gap-2 py-6 text-gray-400 dark:text-gray-500">
      <div className="w-7 h-7 border-2 border-current border-t-transparent rounded-full animate-spin" />
      <p className="text-xs">Fetching usage data…</p>
    </div>
  );
}

function ErrorState({ error, onRetry }: { error: string; onRetry: () => void }) {
  return (
    <div className="flex flex-col items-center gap-3 text-center py-4 px-3">
      <div className="w-10 h-10 bg-red-100 dark:bg-red-900/30 rounded-full flex items-center justify-center">
        <span className="text-red-500 text-xl">!</span>
      </div>
      <div>
        <p className="text-xs font-medium text-gray-800 dark:text-gray-200 mb-0.5">Failed to fetch</p>
        <p className="text-[10px] text-gray-500 dark:text-gray-400 break-all">{error}</p>
      </div>
      <button onClick={onRetry}
        className="px-4 py-1.5 text-xs font-medium bg-gray-700 hover:bg-gray-800 text-white rounded-lg transition-colors">
        Retry
      </button>
    </div>
  );
}

function RefreshIcon({ spinning }: { spinning: boolean }) {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor"
      strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"
      className={spinning ? "animate-spin" : ""}>
      <path d="M23 4v6h-6" />
      <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
    </svg>
  );
}

function SettingsIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor"
      strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06-.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
    </svg>
  );
}

function LogoutIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor"
      strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
      <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
      <polyline points="16 17 21 12 16 7" />
      <line x1="21" y1="12" x2="9" y2="12" />
    </svg>
  );
}

function formatTime(iso: string): string {
  try {
    return new Date(iso).toLocaleTimeString(undefined, {
      hour: "2-digit", minute: "2-digit", second: "2-digit",
    });
  } catch {
    return iso;
  }
}
