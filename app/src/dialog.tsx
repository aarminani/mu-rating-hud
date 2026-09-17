import { useEffect, useRef, useState } from "react";
import { CandyButton } from "@/components/ui/candy-button";
import { TrashIcon } from "./components/Icons";

export interface DialogSpec {
  title: string;
  body?: string;
  field?: { placeholder?: string; value?: string };
  choices?: { value: string; label: string; hint?: string }[];
  custom?: { placeholder?: string };
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
  acknowledge?: boolean;
}

type Pending = { spec: DialogSpec; settle: (v: string | null) => void };

let publish: ((p: Pending | null) => void) | null = null;
let queued: Pending | null = null;

function open(spec: DialogSpec): Promise<string | null> {
  return new Promise((settle) => {
    const p = { spec, settle };
    if (publish) publish(p);
    else queued = p;
  });
}

export async function askText(spec: DialogSpec & { field: NonNullable<DialogSpec["field"]> }) {
  const v = await open(spec);
  return v === null ? null : v.trim();
}

export async function askChoice(
  spec: DialogSpec & { choices: NonNullable<DialogSpec["choices"]> }
) {
  return open({ ...spec, field: { value: spec.field?.value ?? "" } });
}

export async function askConfirm(spec: DialogSpec) {
  return (await open(spec)) !== null;
}

export async function showAlert(spec: DialogSpec) {
  await open({ ...spec, acknowledge: true });
}

export function DialogHost() {
  const [pending, setPending] = useState<Pending | null>(queued);
  const [draft, setDraft] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    publish = setPending;
    if (queued) {
      setPending(queued);
      queued = null;
    }
    return () => {
      publish = null;
    };
  }, []);

  useEffect(() => {
    if (pending) {
      setDraft(pending.spec.field?.value ?? "");
      if (!pending.spec.choices) {
        requestAnimationFrame(() => inputRef.current?.select());
      }
    }
  }, [pending]);

  if (!pending) return null;
  const { spec } = pending;

  const close = (value: string | null) => {
    pending.settle(value);
    setPending(null);
  };

  const accept = () => close(spec.field ? draft : "");

  return (
    <div
      className="dialog-scrim"
      onMouseDown={(e) => e.target === e.currentTarget && close(null)}
    >
      <div className="dialog" role="dialog" aria-modal="true" aria-label={spec.title}>
        <div className="dialog-head">{spec.title}</div>
        {spec.body && <p className="dialog-body">{spec.body}</p>}

        <form
          onSubmit={(e) => {
            e.preventDefault();
            accept();
          }}
        >
          {spec.choices && (
            <div className="dialog-choices">
              {spec.choices.map((c) => (
                <button
                  key={c.value}
                  type="button"
                  className={`dialog-choice ${draft === c.value ? "on" : ""}`}
                  onClick={() => setDraft(c.value)}
                  onDoubleClick={() => close(c.value)}
                >
                  <span className="choice-label">{c.label}</span>
                  {c.hint && <span className="choice-hint">{c.hint}</span>}
                </button>
              ))}
            </div>
          )}
          {spec.field && (!spec.choices || spec.custom) && (
            <input
              ref={inputRef}
              className="dialog-input"
              autoFocus={!spec.choices}
              placeholder={spec.custom?.placeholder ?? spec.field.placeholder}
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              onKeyDown={(e) => e.key === "Escape" && close(null)}
            />
          )}

          <div className="dialog-actions">
            {!spec.acknowledge && (
              <button type="button" onClick={() => close(null)}>
                {spec.cancelLabel ?? "Cancel"}
              </button>
            )}
            {spec.danger ? (
              <CandyButton type="submit" tone="danger" autoFocus>
                <TrashIcon />
                {spec.confirmLabel ?? "Delete"}
              </CandyButton>
            ) : (
              <button
                type="submit"
                className="primary"
                disabled={!!spec.field && !spec.choices && draft.trim() === ""}
                autoFocus={!spec.field}
              >
                {spec.confirmLabel ?? "OK"}
              </button>
            )}
          </div>
        </form>
      </div>
    </div>
  );
}
