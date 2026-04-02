import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import { useUsage } from "./hooks/useUsage";
import UsageCard from "./components/UsageCard";

export default function App() {
  const { usage, loading, error, isLoggedIn, refetch } = useUsage();

  // Hide window when it loses focus
  useEffect(() => {
    const win = getCurrentWindow();
    const unlisten = win.onFocusChanged(({ payload: focused }) => {
      if (!focused) win.hide();
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  async function handleLogout() {
    try {
      await invoke("logout");
    } catch (_) {}
  }

  return (
    <div className="w-[400px] h-[600px] flex flex-col bg-white dark:bg-[#1c1c1e] overflow-hidden rounded-2xl shadow-2xl">
      <MainView
        usage={usage}
        loading={loading}
        error={error}
        isLoggedIn={isLoggedIn}
        onRefresh={refetch}
        onLogout={handleLogout}
      />
    </div>
  );
}

interface MainViewProps {
  usage: ReturnType<typeof useUsage>["usage"];
  loading: boolean;
  error: string | null;
  isLoggedIn: boolean;
  onRefresh: () => void;
  onLogout: () => void;
}

function MainView({ usage, loading, error, isLoggedIn, onRefresh, onLogout }: MainViewProps) {
  const planBadge = usage?.plan_type ?? null;

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between px-5 py-4 border-b border-black/10 dark:border-white/10">
        <div className="flex items-center gap-2">
          <div className="w-6 h-6 bg-gradient-to-br from-orange-400 to-red-500 rounded-md flex items-center justify-center">
            <span className="text-white text-xs font-bold">C</span>
          </div>
          <div>
            <h1 className="text-base font-semibold text-gray-900 dark:text-gray-100 leading-tight">
              Claude Usage
            </h1>
            {planBadge && (
              <span className="text-[10px] font-medium text-orange-500 dark:text-orange-400 uppercase tracking-wider">
                {planBadge}
              </span>
            )}
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={onRefresh}
            disabled={loading}
            title="Refresh"
            className="w-7 h-7 flex items-center justify-center rounded-full bg-black/5 dark:bg-white/5 hover:bg-black/10 dark:hover:bg-white/10 text-gray-600 dark:text-gray-400 transition-colors disabled:opacity-40"
          >
            <RefreshIcon spinning={loading} />
          </button>
          {isLoggedIn && (
            <button
              onClick={onLogout}
              title="Sign out"
              className="w-7 h-7 flex items-center justify-center rounded-full bg-black/5 dark:bg-white/5 hover:bg-red-100 dark:hover:bg-red-900/30 text-gray-600 dark:text-gray-400 hover:text-red-500 dark:hover:text-red-400 transition-colors"
            >
              <LogoutIcon />
            </button>
          )}
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 flex flex-col items-center justify-center px-5 py-6 gap-6">
        {error ? (
          <ErrorState error={error} onRetry={onRefresh} />
        ) : !isLoggedIn ? (
          <NotLoggedInPrompt />
        ) : loading && !usage ? (
          <LoadingState />
        ) : usage ? (
          <>
            <p className="text-xs text-gray-500 dark:text-gray-400 font-medium uppercase tracking-wider">
              Current Usage
            </p>
            <div className="flex gap-8 items-center justify-center">
              <UsageCard label="5-Hour" usage={usage.five_hour} />
              <div className="w-px h-20 bg-black/10 dark:bg-white/10" />
              <UsageCard label="7-Day" usage={usage.seven_day} />
            </div>

            <div className="w-full space-y-3 mt-2">
              <UsageBar label="5-Hour Period" percent={usage.five_hour.utilization} />
              <UsageBar label="7-Day Period" percent={usage.seven_day.utilization} />
            </div>
          </>
        ) : (
          <LoadingState />
        )}
      </div>

      {/* Footer */}
      <div className="px-5 py-3 border-t border-black/10 dark:border-white/10">
        <p className="text-xs text-gray-400 dark:text-gray-500 text-center">
          Claude Pro / Max Usage Monitor
        </p>
      </div>
    </div>
  );
}

function NotLoggedInPrompt() {
  async function openLogin() {
    try {
      await invoke("open_login_window");
    } catch (_) {}
  }

  return (
    <div className="flex flex-col items-center gap-4 text-center px-4">
      <div className="w-14 h-14 bg-orange-100 dark:bg-orange-900/30 rounded-full flex items-center justify-center">
        <span className="text-3xl">☁</span>
      </div>
      <div>
        <p className="text-sm font-semibold text-gray-800 dark:text-gray-200 mb-1">
          Connect your Claude account
        </p>
        <p className="text-xs text-gray-500 dark:text-gray-400">
          Log in to monitor your Pro / Max plan usage in real time.
        </p>
      </div>
      <button
        onClick={openLogin}
        className="px-5 py-2.5 text-sm font-medium bg-[#d97706] hover:bg-[#b45309] text-white rounded-xl transition-colors"
      >
        Open Claude.ai Login
      </button>
    </div>
  );
}

function UsageBar({ label, percent }: { label: string; percent: number }) {
  const color =
    percent >= 80 ? "bg-red-500" : percent >= 50 ? "bg-yellow-500" : "bg-emerald-500";

  return (
    <div className="space-y-1">
      <div className="flex justify-between items-center">
        <span className="text-xs font-medium text-gray-600 dark:text-gray-400">{label}</span>
        <span className="text-xs font-semibold text-gray-800 dark:text-gray-200">
          {Math.round(percent)}%
        </span>
      </div>
      <div className="h-2 bg-black/10 dark:bg-white/10 rounded-full overflow-hidden">
        <div
          className={`h-full rounded-full transition-all duration-700 ${color}`}
          style={{ width: `${Math.min(percent, 100)}%` }}
        />
      </div>
    </div>
  );
}

function LoadingState() {
  return (
    <div className="flex flex-col items-center gap-3 text-gray-400 dark:text-gray-500">
      <div className="w-8 h-8 border-2 border-current border-t-transparent rounded-full animate-spin" />
      <p className="text-sm">Fetching usage data...</p>
    </div>
  );
}

function ErrorState({ error, onRetry }: { error: string; onRetry: () => void }) {
  return (
    <div className="flex flex-col items-center gap-4 text-center px-4">
      <div className="w-12 h-12 bg-red-100 dark:bg-red-900/30 rounded-full flex items-center justify-center">
        <span className="text-red-500 text-2xl">!</span>
      </div>
      <div>
        <p className="text-sm font-medium text-gray-800 dark:text-gray-200 mb-1">
          Failed to fetch usage
        </p>
        <p className="text-xs text-gray-500 dark:text-gray-400 break-all">{error}</p>
      </div>
      <button
        onClick={onRetry}
        className="px-4 py-2 text-sm font-medium bg-gray-700 hover:bg-gray-800 text-white rounded-lg transition-colors"
      >
        Retry
      </button>
    </div>
  );
}

function RefreshIcon({ spinning }: { spinning: boolean }) {
  return (
    <svg
      width="14"
      height="14"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={spinning ? "animate-spin" : ""}
    >
      <path d="M23 4v6h-6" />
      <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
    </svg>
  );
}

function LogoutIcon() {
  return (
    <svg
      width="14"
      height="14"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.5"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
      <polyline points="16 17 21 12 16 7" />
      <line x1="21" y1="12" x2="9" y2="12" />
    </svg>
  );
}
