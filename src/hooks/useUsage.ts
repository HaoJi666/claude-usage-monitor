import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface PeriodUsage {
  utilization: number;
  resets_at: string;
  kind?: string; // "five_hour" | "current_session" | "session"
}

export interface ExtraUsage {
  enabled: boolean;
  spent: number;
  limit: number;
  balance: number;
  percent_used: number;
  resets_at: string;
  auto_reload: boolean;
}

export interface UsageData {
  five_hour: PeriodUsage;
  seven_day: PeriodUsage;
  seven_day_sonnet?: PeriodUsage | null;
  plan_type?: string | null;
  extra_usage?: ExtraUsage | null;
  fetched_at: string;
}

interface LoginStatus {
  is_logged_in: boolean;
  email: string | null;
  plan_type: string | null;
}

export function useUsage() {
  const [usage, setUsage] = useState<UsageData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isLoggedIn, setIsLoggedIn] = useState(false);
  const unlistenUsage = useRef<(() => void) | null>(null);
  const unlistenLogin = useRef<(() => void) | null>(null);
  const loadingTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const fetchUsage = useCallback(async () => {
    if (loadingTimer.current) clearTimeout(loadingTimer.current);
    setLoading(true);
    setError(null);
    // Show cached data immediately while the real refresh is in flight.
    try {
      const cached = await invoke<UsageData | null>("get_usage");
      if (cached) setUsage(cached);
    } catch (e) {
      setError(String(e));
    }
    // Navigate session window to /settings/usage to re-fetch live data.
    // Result arrives via the usage-updated event which stops the spinner.
    invoke("trigger_refresh").catch(() => {});
    // Fallback: stop spinner after 8 s in case usage-updated never fires.
    loadingTimer.current = setTimeout(() => setLoading(false), 8000);
  }, []);

  useEffect(() => {
    invoke<LoginStatus>("get_login_status")
      .then((s) => setIsLoggedIn(s.is_logged_in))
      .catch(() => {});
    fetchUsage();

    listen<UsageData>("usage-updated", (event) => {
      setUsage(event.payload);
      setError(null);
      setLoading(false);
      if (loadingTimer.current) clearTimeout(loadingTimer.current);
    }).then((fn) => { unlistenUsage.current = fn; });

    listen<boolean>("login-status-changed", (event) => {
      setIsLoggedIn(event.payload);
      if (!event.payload) setUsage(null);
    }).then((fn) => { unlistenLogin.current = fn; });

    return () => {
      unlistenUsage.current?.();
      unlistenLogin.current?.();
      if (loadingTimer.current) clearTimeout(loadingTimer.current);
    };
  }, [fetchUsage]);

  return { usage, loading, error, isLoggedIn, refetch: fetchUsage };
}
