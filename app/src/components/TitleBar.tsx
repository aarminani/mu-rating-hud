import { getCurrentWindow } from "@tauri-apps/api/window";
import { Logo, Wordmark } from "./Brand";

export function TitleBar() {
  const win = () => getCurrentWindow();

  return (
    <div className="titlebar" data-tauri-drag-region>
      <span className="brand" data-tauri-drag-region>
        <Logo size={20} />
        <Wordmark />
      </span>

      <span className="titlebar-fill" data-tauri-drag-region />

      <div className="window-controls">
        <button
          className="win-btn"
          aria-label="Minimize"
          onClick={() => win().minimize()}
        >
          <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden>
            <rect x="0" y="4.5" width="10" height="1" fill="currentColor" />
          </svg>
        </button>
        <button
          className="win-btn close"
          aria-label="Close"
          onClick={() => win().close()}
        >
          <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden>
            <path d="M0 0L10 10M10 0L0 10" stroke="currentColor" fill="none" />
          </svg>
        </button>
      </div>
    </div>
  );
}
