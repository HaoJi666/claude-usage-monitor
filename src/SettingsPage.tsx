import { getCurrentWindow } from "@tauri-apps/api/window";
import Settings from "./components/Settings";
import "./App.css";

export default function SettingsPage() {
  function handleClose() {
    getCurrentWindow().close().catch(() => {});
  }

  return (
    <div className="w-full min-h-screen bg-white dark:bg-[#1c1c1e]">
      <Settings onClose={handleClose} />
    </div>
  );
}
