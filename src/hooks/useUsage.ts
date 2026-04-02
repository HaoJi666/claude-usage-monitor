import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface PeriodUsage {
  utilization: number;
  resets_at: string;
}

export interface UsageData {
  five_hour: PeriodUsage;
  seven_day: PeriodUsage;
  plan_type?: string | null;
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

  const fetchUsage = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await invoke<UsageData | null>("get_usage");
      if (data) setUsage(data);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    // Load initial login state and usage cache
    invoke<LoginStatus>("get_login_status")
      .then((s) => setIsLoggedIn(s.is_logged_in))
      .catch(() => {});
    fetchUsage();

    // Real-time usage updates from the session webview
    listen<UsageData>("usage-updated", (event) => {
      setUsage(event.payload);
      setError(null);
    }).then((fn) => { unlistenUsage.current = fn; });

    // Login state changes from the session webview
    listen<boolean>("login-status-changed", (event) => {
      setIsLoggedIn(event.payload);
      if (!event.payload) {
        setUsage(null);
      }
    }).then((fn) => { unlistenLogin.current = fn; });

    return () => {
      unlistenUsage.current?.();
      unlistenLogin.current?.();
    };
  }, [fetchUsage]);

  return { usage, loading, error, isLoggedIn, refetch: fetchUsage };
}
