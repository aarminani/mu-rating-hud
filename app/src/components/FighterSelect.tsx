import { useEffect, useState, type CSSProperties } from "react";

import type { AccountRow, ConnectProgress } from "../lib/types";
import { monogram } from "../lib/format";
import { portraitFor } from "../lib/portraits";
import { Logo } from "./Brand";
import { Button } from "./ui/button";
import { CheckIcon, ChevronLeftIcon, ChevronRightIcon, PlusIcon } from "./Icons";
import { MAX_ACCOUNTS, prettyId } from "./ConnectList";

const FOLLOWED_LABEL = "The HUD follows this account";

const VISIBLE = 4;

export function FighterSelect({
  accounts,
  addRow,
  busy,
  progress,
  error,
  cancelLabel,
  onCancel,
  onConnect,
  onDisconnect,
}: {
  accounts: AccountRow[];
  addRow: boolean;
  busy: boolean;
  progress: Record<string, ConnectProgress>;
  error: string | null;
  cancelLabel: "Cancel" | "Exit";
  onCancel: () => void;
  onConnect: (ids: string[], prefer: string) => void;
  onDisconnect: () => void;
}) {
  const [removed, setRemoved] = useState<string[]>([]);
  const [adding, setAdding] = useState((addRow && accounts.length < MAX_ACCOUNTS) || accounts.length === 0);
  const [newId, setNewId] = useState("");
  const kept = accounts.filter((a) => !removed.includes(a.tekken_id));
  const followed = kept.find((a) => a.active) ?? kept.find((a) => a.signed_in);
  const [selected, setSelected] = useState<string | null>(() =>
    adding ? null : (followed ?? kept[0])?.tekken_id ?? null,
  );

  const tail = kept.length < MAX_ACCOUNTS ? 1 : 0;
  const count = Math.max(VISIBLE, kept.length + tail);
  const maxOffset = count - VISIBLE;
  const [offset, setOffset] = useState(() => {
    const start = adding ? kept.length : Math.max(0, kept.findIndex((a) => a.tekken_id === selected));
    return Math.min(Math.max(0, start - (VISIBLE - 1)), maxOffset);
  });
  const at = Math.min(offset, maxOffset);
  const reveal = (i: number) =>
    setOffset((o) => {
      const cur = Math.min(o, maxOffset);
      return i < cur ? i : i > cur + VISIBLE - 1 ? i - (VISIBLE - 1) : cur;
    });

  useEffect(() => {
    if (!adding && selected && !kept.some((a) => a.tekken_id === selected)) {
      setSelected(kept[0]?.tekken_id ?? null);
      setOffset(0);
      if (kept.length === 0) setAdding(true);
    }
  }, [kept, selected, adding]);

  const chosen = adding ? null : kept.find((a) => a.tekken_id === selected) ?? null;
  const bareNew = newId.replace(/[\s-]/g, "");
  const ids = [...kept.map((a) => a.tekken_id), ...(adding && bareNew ? [bareNew] : [])];
  const prefer = adding ? bareNew : chosen?.tekken_id ?? "";
  const canConnect = !busy && ids.length > 0 && prefer !== "";
  const connect = () => canConnect && onConnect(ids, prefer);
  const newProgress = progress[bareNew];

  return (
    <div className="connect fighter" data-tauri-drag-region>
      <div className="connect-mark" data-tauri-drag-region>
        <Logo size={96} />
      </div>
      <div className="connect-copy" data-tauri-drag-region>
        <h1>{busy ? "Connecting…" : "Choose your account"}</h1>
        <p>
          {kept.length === 0
            ? "Add your Tekken ID to see your rating in game."
            : followed
              ? "The HUD follows the account with the check mark."
              : "Pick the account for the HUD to follow."}
        </p>
        {error && <p className="connect-error fs-error">{error}</p>}
      </div>

      <div
        className={`fs-roster${maxOffset === 0 ? " fits" : ""}`}
        key={`${adding ? "roster-adding" : "roster-chosen"}-${removed.length}`}
      >
        <button
          className="fs-arrow prev"
          aria-label="Previous accounts"
          disabled={at === 0}
          onClick={() => setOffset(Math.max(0, at - 1))}
        >
          <ChevronLeftIcon size={12} />
        </button>
        <div className="fs-viewport">
          <div className="fs-track" style={{ "--at": at } as CSSProperties}>
        {Array.from({ length: count }, (_, i) => {
          const a = kept[i];
          if (a) {
            const sel = !adding && a.tekken_id === selected;
            const art = portraitFor(a.character);
            return (
              <button
                key={a.tekken_id}
                className={`fs-card${sel ? " sel" : ""}${art ? " has-art" : ""}`}
                disabled={busy}
                onFocus={() => reveal(i)}
                onClick={() => {
                  setAdding(false);
                  setSelected(a.tekken_id);
                }}
              >
                {art ? (
                  <img className="fs-art" src={art} alt="" draggable={false} decoding="async" />
                ) : (
                  <span className="fs-mono">{monogram(a.character ?? a.name)}</span>
                )}
                {a === followed && (
                  <span className="fs-check" title={FOLLOWED_LABEL} aria-label={FOLLOWED_LABEL} role="img">
                    <CheckIcon size={13} />
                  </span>
                )}
                <span className="fs-name">{a.name}</span>
                <span className="fs-mu">&#956; {a.mu ?? "—"}</span>
              </button>
            );
          }
          if (i === kept.length) {
            return adding ? (
              <div key="new" className="fs-card sel">
                <span className="fs-mono">{bareNew ? monogram(bareNew) : "?"}</span>
                <span className="fs-name">New</span>
                <span className="fs-mu">&#956; {"—"}</span>
              </div>
            ) : (
              <button
                key="add"
                className="fs-card add"
                disabled={busy}
                onFocus={() => reveal(i)}
                onClick={() => setAdding(true)}
              >
                <PlusIcon size={18} />
                Add
              </button>
            );
          }
          return <div key={`blank${i}`} className="fs-card blank" />;
        })}
          </div>
        </div>
        <button
          className="fs-arrow next"
          aria-label="More accounts"
          disabled={at === maxOffset}
          onClick={() => setOffset(Math.min(maxOffset, at + 1))}
        >
          <ChevronRightIcon size={12} />
        </button>
      </div>

      <div className="fs-detail" key={adding ? "detail-adding" : "detail-chosen"}>
        {adding ? (
          <>
            <span className="faint">Tekken ID</span>
            <input
              className="acct-input"
              aria-label="Tekken ID"
              value={newId}
              autoFocus
              disabled={busy}
              onChange={(e) => setNewId(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && connect()}
              placeholder="3zi8-CzQf-sE9b"
              spellCheck={false}
              autoComplete="off"
              maxLength={16}
            />
            {busy && newProgress?.state === "reading" && <span className="faint">Reading Wavu page&hellip;</span>}
          </>
        ) : chosen ? (
          <>
            <b>{chosen.name}</b>
            <span>
              &middot; {chosen.character ? `${chosen.character}, highest rating` : "no rating yet"}
            </span>
            <span className="acct-id">{prettyId(chosen.tekken_id)}</span>
          </>
        ) : (
          <span className="faint">Pick a card, or add an account.</span>
        )}
      </div>

      <div className="connect-actions">
        <Button
          variant="secondary"
          disabled={busy || (adding ? false : !chosen)}
          title={adding ? undefined : "Removed when you press Connect"}
          onClick={() => {
            if (adding) {
              setNewId("");
              if (kept.length > 0) setAdding(false);
            } else if (chosen) {
              setRemoved((r) => [...r, chosen.tekken_id]);
            }
          }}
        >
          Remove
        </Button>
        {accounts.length > 0 && kept.length === 0 && !bareNew ? (
          <Button variant="brand" loading={busy} disabled={busy} onClick={onDisconnect}>
            Disconnect
          </Button>
        ) : (
          <Button variant="brand" loading={busy} disabled={!canConnect} onClick={connect}>
            Connect
          </Button>
        )}
        <button onClick={onCancel}>{cancelLabel}</button>
      </div>
    </div>
  );
}
