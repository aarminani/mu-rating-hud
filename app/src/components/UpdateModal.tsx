import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Button } from "./ui/button";

export interface Available {
  version: string;
  running: string;
  page: string;
  asset: string | null;
  size: number;
  exe: string | null;
  exe_size: number;
}

const IDLE_MS = 30_000;
const COUNTDOWN_S = 10;

function megabytes(bytes: number): string {
  return `${(bytes / 1_000_000).toFixed(1)} MB`;
}

export function UpdateModal() {
  const [found, setFound] = useState<Available | null>(null);
  const [left, setLeft] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const staying = useRef(false);
  const raised = useRef(false);

  useEffect(() => {
    const show = (u: Available) => {
      if (raised.current) return;
      raised.current = true;
      setFound(u);
    };
    const un = listen<Available>("update:available", (e) => show(e.payload));
    invoke<Available | null>("update_available")
      .then((u) => u && show(u))
      .catch(() => {});
    return () => {
      un.then((f) => f()).catch(() => {});
    };
  }, []);

  useEffect(() => {
    if (!found) return;
    const idle = window.setTimeout(() => {
      if (!staying.current) setLeft(COUNTDOWN_S);
    }, IDLE_MS);
    return () => window.clearTimeout(idle);
  }, [found]);

  useEffect(() => {
    if (left === null || staying.current) return;
    if (left <= 0) {
      close();
      return;
    }
    const t = window.setTimeout(() => setLeft((n) => (n === null ? null : n - 1)), 1000);
    return () => window.clearTimeout(t);
  }, [left]);

  function close() {
    setFound(null);
    setLeft(null);
    invoke("update_dismissed").catch(() => {});
  }

  function stay() {
    staying.current = true;
    setLeft(null);
  }

  if (!found) return null;

  const install = async () => {
    stay();
    if (!found.exe) {
      openUrl(found.page).catch(() => {});
      close();
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await invoke("update_install");
    } catch (e) {
      setBusy(false);
      setError(typeof e === "string" ? e : "the update could not be installed");
    }
  };

  return (
    <UpdateCard
      found={found}
      left={left}
      busy={busy}
      error={error}
      onStay={stay}
      onLater={() => {
        stay();
        close();
      }}
      onDownload={install}
      onOpenPage={() => {
        openUrl(found.page).catch(() => {});
        close();
      }}
    />
  );
}

export function UpdateCard({
  found,
  left,
  busy,
  error,
  onStay,
  onLater,
  onDownload,
  onOpenPage,
}: {
  found: Available;
  left: number | null;
  busy?: boolean;
  error?: string | null;
  onStay: () => void;
  onLater: () => void;
  onDownload: () => void;
  onOpenPage: () => void;
}) {
  const canInstall = Boolean(found.exe);
  const size = found.exe_size || found.size;
  return (
    <div className="update-scrim" onPointerDown={onStay} onPointerMove={onStay} onKeyDown={onStay}>
      <div className="update-card" role="dialog" aria-modal="true" aria-label="Update available">
        <div className="update-head">Mu Rating {found.version.replace(/^v/i, "")} is available</div>
        <p className="update-body">
          {error ? (
            <>
              That did not work: {error}. You can still update by hand, the page has the download
              and the steps.
            </>
          ) : canInstall ? (
            <>
              You are on {found.running}. Update installs it and restarts the helper, which takes a
              few seconds. Your accounts, settings and achievements are kept.
            </>
          ) : (
            <>
              You are on {found.running}. The update is a new <code>trhmu.exe</code> to unzip over
              your current one; your accounts and settings are kept.
            </>
          )}
          {!error && size > 0 && <span className="update-size"> {megabytes(size)}</span>}
        </p>
        <div className="update-actions">
          {left !== null && !busy && (
            <span className="update-count" aria-live="polite">
              Closing in {left}
            </span>
          )}
          {busy && (
            <span className="update-count" aria-live="polite">
              Downloading and installing...
            </span>
          )}
          {!busy && (
            <Button variant="ghost" onClick={onLater}>
              Later
            </Button>
          )}
          {error ? (
            <Button variant="brand" onClick={onOpenPage}>
              Open the page
            </Button>
          ) : (
            <Button variant="brand" onClick={onDownload} disabled={busy}>
              {busy ? "Updating" : canInstall ? "Update" : "Download"}
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
