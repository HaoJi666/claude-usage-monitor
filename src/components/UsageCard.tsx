import { useEffect, useState } from "react";
import CircularProgress from "./CircularProgress";

interface PeriodUsage {
  utilization: number;
  resets_at: string;
}

interface UsageCardProps {
  label: string;
  usage: PeriodUsage;
}

function formatCountdown(resetsAt: string): string {
  const now = new Date();
  const reset = new Date(resetsAt);
  const diffMs = reset.getTime() - now.getTime();

  if (diffMs <= 0) return "Resetting...";

  const totalSecs = Math.floor(diffMs / 1000);
  const days = Math.floor(totalSecs / 86400);
  const hours = Math.floor((totalSecs % 86400) / 3600);
  const mins = Math.floor((totalSecs % 3600) / 60);

  if (days > 0) return `${days}d ${hours}h ${mins}m`;
  if (hours > 0) return `${hours}h ${mins}m`;
  return `${mins}m`;
}

export default function UsageCard({ label, usage }: UsageCardProps) {
  const [countdown, setCountdown] = useState(() => formatCountdown(usage.resets_at));

  useEffect(() => {
    setCountdown(formatCountdown(usage.resets_at));
    const timer = setInterval(() => {
      setCountdown(formatCountdown(usage.resets_at));
    }, 30000);
    return () => clearInterval(timer);
  }, [usage.resets_at]);

  return (
    <div className="flex flex-col items-center gap-3 p-4 rounded-2xl bg-black/5 dark:bg-white/5">
      <CircularProgress
        percent={usage.utilization}
        size={110}
        strokeWidth={9}
        label={label}
        sublabel={`Resets in ${countdown}`}
      />
    </div>
  );
}
