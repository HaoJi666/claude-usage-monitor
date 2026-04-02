interface CircularProgressProps {
  percent: number;  // 0-100
  size?: number;
  strokeWidth?: number;
  label?: string;
  sublabel?: string;
}

function getColor(percent: number): string {
  if (percent >= 80) return "#ef4444"; // red
  if (percent >= 50) return "#f59e0b"; // yellow
  return "#10b981";                     // green
}

export default function CircularProgress({
  percent,
  size = 100,
  strokeWidth = 8,
  label,
  sublabel,
}: CircularProgressProps) {
  const radius = (size - strokeWidth) / 2;
  const circumference = 2 * Math.PI * radius;
  const offset = circumference - (percent / 100) * circumference;
  const color = getColor(percent);
  const cx = size / 2;
  const cy = size / 2;

  return (
    <div className="flex flex-col items-center gap-1">
      <div className="relative" style={{ width: size, height: size }}>
        <svg width={size} height={size} style={{ transform: "rotate(-90deg)" }}>
          {/* Track */}
          <circle
            cx={cx}
            cy={cy}
            r={radius}
            fill="none"
            stroke="currentColor"
            strokeWidth={strokeWidth}
            className="text-black/10 dark:text-white/10"
          />
          {/* Progress */}
          <circle
            cx={cx}
            cy={cy}
            r={radius}
            fill="none"
            stroke={color}
            strokeWidth={strokeWidth}
            strokeLinecap="round"
            strokeDasharray={circumference}
            strokeDashoffset={offset}
            className="progress-circle"
          />
        </svg>
        {/* Center text */}
        <div className="absolute inset-0 flex items-center justify-center">
          <span className="text-lg font-bold" style={{ color }}>
            {Math.round(percent)}%
          </span>
        </div>
      </div>
      {label && (
        <div className="text-center">
          <p className="text-sm font-medium text-gray-800 dark:text-gray-200">{label}</p>
          {sublabel && (
            <p className="text-xs text-gray-500 dark:text-gray-400">{sublabel}</p>
          )}
        </div>
      )}
    </div>
  );
}
