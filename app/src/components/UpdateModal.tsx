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
}

const IDLE_MS = 30_000;
const COUNTDOWN_S = 10;

function megabytes(bytes: number): string {
  return `${(bytes / 1_000_000).toFixed(1)} MB`;
}

export function UpdateModal() {
  const [found, setFound] = useState<Available | null>(null);
  const [left, setLeft] = useState<number | null>(null);
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

  return (
    <UpdateCard
      found={found}
      left={left}
      onStay={stay}
      onLater={() => {
        stay();
        close();
      }}
      onDownload={() => {
        stay();
        openUrl(found.page).catch(() => {});
        close();
      }}
    />
  );
}

export function UpdateCard({
  found,
  left,
  onStay,
  onLater,
  onDownload,
}: {
  found: Available;
  left: number | null;
  onStay: () => void;
  onLater: () => void;
  onDownload: () => void;
}) {
  return (
    <div className="update-scrim" onPointerDown={onStay} onPointerMove={onStay} onKeyDown={onStay}>
      <div className="update-card" role="dialog" aria-modal="true" aria-label="Update available">
        <div className="update-head">Mu Rating {found.version.replace(/^v/i, "")} is available</div>
        <p className="update-body">
          You are on {found.running}. The update is a new <code>trhmu.exe</code> to unzip over your
          current one; your accounts and settings are kept.
          {found.size > 0 && <span className="update-size"> {megabytes(found.size)}</span>}
        </p>
        <div className="update-actions">
          {left !== null && (
            <span className="update-count" aria-live="polite">
              Closing in {left}
            </span>
          )}
          <Button variant="ghost" onClick={onLater}>
            Later
          </Button>
          <Button variant="brand" onClick={onDownload}>
            Download
          </Button>
        </div>
      </div>
    </div>
  );
}
