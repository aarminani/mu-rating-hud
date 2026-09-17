import { useEffect, useRef, useState } from "react";

import type { AccountRow, ConnectProgress } from "../lib/types";
import { Logo, DotsRing } from "./Brand";
import { Button } from "./ui/button";
import { CheckIcon, CloseIcon, PlusIcon } from "./Icons";

export const MAX_ACCOUNTS = 5;

export const prettyId = (id: string) => id.replace(/[\s-]/g, "").replace(/(.{4})(?=.)/g, "$1-");

export type Row = { key: number; id: string; saved: AccountRow | null };

let nextKey = 1;
export const newRow = (id = ""): Row => ({ key: nextKey++, id, saved: null });

export function initialRows(accounts: AccountRow[], addRow: boolean, legacy: string[]): Row[] {
  const rows: Row[] = accounts.map((a) => ({ key: nextKey++, id: a.tekken_id, saved: a }));
  if (rows.length === 0) {
    const typed = legacy.map((s) => s.trim()).filter(Boolean).slice(0, MAX_ACCOUNTS);
    return typed.length ? typed.map((id) => newRow(id)) : [newRow()];
  }
  if (addRow && rows.length < MAX_ACCOUNTS) rows.push(newRow());
  return rows;
}

export function ConnectList({
  accounts,
  addRow,
  legacy,
  busy,
  progress,
  error,
  cancelLabel,
  onCancel,
  onConnect,
  onDisconnect,
  fitRef,
}: {
  accounts: AccountRow[];
  addRow: boolean;
  legacy: string[];
  busy: boolean;
  progress: Record<string, ConnectProgress>;
  error: string | null;
  cancelLabel: "Cancel" | "Exit";
  onCancel: () => void;
  onConnect: (ids: string[]) => void;
  onDisconnect: () => void;
  fitRef: React.Ref<HTMLDivElement>;
}) {
  const [rows, setRows] = useState<Row[]>(() => initialRows(accounts, addRow, legacy));
  const lastInput = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    setRows((prev) =>
      prev.map((r) => (r.saved ? { ...r, saved: accounts.find((a) => a.tekken_id === r.saved!.tekken_id) ?? r.saved } : r)),
    );
  }, [accounts]);

  const filled = rows.filter((r) => r.id.trim());
  const n = filled.length;
  const noun = n === 1 ? "account" : "accounts";
  const firstRun = accounts.length === 0;
  const connect = () => n > 0 && !busy && onConnect(filled.map((r) => r.id.trim()));

  return (
    <div className="connect" data-tauri-drag-region ref={fitRef}>
      <div className="connect-mark" data-tauri-drag-region>
        <Logo size={64} />
      </div>

      <div className="connect-copy">
        {busy ? (
          <>
            <h1>
              Connecting {n} {noun}
            </h1>
            <p>Wavu asks for 10 seconds between pages, so new IDs take a moment.</p>
          </>
        ) : error ? (
          <>
            <h1>Could not connect</h1>
            <p className="connect-error">{error}</p>
          </>
        ) : firstRun ? (
          <>
            <h1>Connect to Wavu Wank</h1>
            <p>This app reads your Mu Rating from Wavu Wank and hands it over to the in-game HUD.</p>
            <p className="connect-reassure">
              Watch your rating change each battle, and see what you&apos;re up against.
            </p>
          </>
        ) : (
          <>
            <h1>Your Tekken accounts</h1>
            <p>The HUD follows whichever Steam account is signed in.</p>
          </>
        )}

        <div className="acct-list">
          <div className="group-head acct-head">
            <span className="group-word">Accounts</span>
            <span className="faint">
              &middot; {rows.length} of {MAX_ACCOUNTS}
            </span>
          </div>
          {rows.map((r, i) => {
            const p = progress[r.id.replace(/[\s-]/g, "")];
            if (busy) {
              if (!r.id.trim()) return null;
              return (
                <div className="acct-row" key={r.key}>
                  {p?.state === "done" ? (
                    <span className="acct-icon ok">
                      <CheckIcon size={13} />
                    </span>
                  ) : p?.state === "reading" ? (
                    <span className="acct-icon">
                      <DotsRing size={13} />
                    </span>
                  ) : p?.state === "failed" ? (
                    <span className="acct-icon bad">
                      <CloseIcon size={13} />
                    </span>
                  ) : (
                    <span className="dot new" />
                  )}
                  {p?.state === "done" ? (
                    <span className="acct-name">{p.name ?? r.saved?.name}</span>
                  ) : (
                    <span className="acct-wait">
                      {p?.state === "reading" ? "Reading Wavu page…" : p?.state === "failed" ? "Could not read" : "Waiting…"}
                    </span>
                  )}
                  <span className="acct-id">{prettyId(r.id)}</span>
                </div>
              );
            }
            return (
              <div className="acct-row" key={r.key}>
                {r.saved ? (
                  <>
                    <span className={`dot ${r.saved.signed_in ? "on" : r.saved.paired ? "off" : "new"}`} />
                    <span className="acct-name">{r.saved.name}</span>
                    {r.saved.signed_in && <span className="badge acct-tag">Signed in</span>}
                    <span className="acct-id">{prettyId(r.saved.tekken_id)}</span>
                  </>
                ) : (
                  <>
                    <span className="dot new" />
                    <input
                      className="acct-input"
                      aria-label={`Tekken ID ${i + 1}`}
                      value={r.id}
                      ref={i === rows.length - 1 ? lastInput : undefined}
                      autoFocus={i === rows.length - 1 && !firstRun}
                      onChange={(e) =>
                        setRows((prev) => prev.map((x) => (x.key === r.key ? { ...x, id: e.target.value } : x)))
                      }
                      onKeyDown={(e) => e.key === "Enter" && connect()}
                      placeholder={i === 0 ? "3zi8-CzQf-sE9b" : "Tekken ID"}
                      spellCheck={false}
                      autoComplete="off"
                      maxLength={16}
                    />
                  </>
                )}
                {(r.saved || rows.length > 1) && (
                  <button
                    className="acct-rm"
                    aria-label="Remove this account"
                    title="Removed when you press Connect"
                    onClick={() => setRows((prev) => prev.filter((x) => x.key !== r.key))}
                  >
                    <CloseIcon size={11} />
                  </button>
                )}
              </div>
            );
          })}
          {!busy && rows.length < MAX_ACCOUNTS && (
            <button
              className="acct-add"
              onClick={() => {
                setRows((prev) => [...prev, newRow()]);
                setTimeout(() => lastInput.current?.focus(), 0);
              }}
            >
              <PlusIcon size={11} />
              Add another account
            </button>
          )}
        </div>
      </div>

      <div className="connect-actions">
        {n === 0 && !firstRun ? (
          <Button variant="brand" loading={busy} disabled={busy} onClick={onDisconnect}>
            Disconnect
          </Button>
        ) : (
          <Button variant="brand" loading={busy} disabled={n === 0 || busy} onClick={connect}>
            {busy ? "Connecting…" : `Connect ${n || ""} ${noun}`.replace("  ", " ")}
          </Button>
        )}
        <button onClick={onCancel}>{cancelLabel}</button>
      </div>
    </div>
  );
}
