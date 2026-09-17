import { useEffect, useRef, useState } from "react";
import { AnimatePresence, motion, useReducedMotion } from "framer-motion";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { Logo } from "./Brand";
import { withEmphasis } from "./Emphasis";
import { RARITY_ICON, RARITY_NAME, rarityVar } from "../lib/rarity";

type ToastPayload = {
  seq: number;
  text: string;
  emphasis: string | null;
  kind?: "milestone" | "achievement";
  rarity?: number;
  title?: string;
};

const DWELL_MS = 4200;

const AWARD_DWELL_MS = 6000;

const OFFSCREEN = 380;

const LINGER_MS = 1500;

let built: Promise<ToastPayload | null> | null = null;
const collect = () => (built ??= invoke<ToastPayload | null>("toast_ready"));

export function Toast() {
  const [payload, setPayload] = useState<ToastPayload | null>(null);
  const timer = useRef<number | null>(null);
  const closer = useRef<number | null>(null);
  const reduced = useReducedMotion();

  useEffect(() => {
    const clear = () => {
      if (timer.current !== null) clearTimeout(timer.current);
      timer.current = null;
      if (closer.current !== null) clearTimeout(closer.current);
      closer.current = null;
    };
    const show = (p: ToastPayload) => {
      clear();
      setPayload(p);
      const dwell = p.rarity ? AWARD_DWELL_MS : DWELL_MS;
      timer.current = window.setTimeout(() => setPayload(null), dwell);
    };

    const un = listen<ToastPayload>("toast:show", (e) => show(e.payload));
    un.then(collect)
      .then((p) => {
        if (p) show(p);
        else getCurrentWindow().close().catch(() => {});
      })
      .catch(() => {});

    return () => {
      clear();
      un.then((f) => f()).catch(() => {});
    };
  }, []);

  const closeSoon = () => {
    if (closer.current !== null) clearTimeout(closer.current);
    closer.current = window.setTimeout(() => getCurrentWindow().close().catch(() => {}), LINGER_MS);
  };

  const travel = reduced ? 0 : OFFSCREEN;
  const rarity = payload?.rarity;

  return (
    <AnimatePresence
      onExitComplete={() => {
        getCurrentWindow().hide().catch(() => {});
        closeSoon();
      }}
    >
      {payload && (
        <motion.div
          key={payload.seq}
          className={`toast-card${rarity ? " award" : ""}`}
          style={rarity ? rarityVar(rarity) : undefined}
          initial={{ x: travel, opacity: 0, scale: reduced ? 1 : 0.97 }}
          animate={{ x: 0, opacity: 1, scale: 1 }}
          exit={{
            x: travel,
            opacity: 0,
            scale: reduced ? 1 : 0.98,
            transition: { duration: 0.26, ease: "easeIn" },
          }}
          transition={{ type: "spring", duration: 0.54, bounce: 0.2 }}
        >
          <motion.span
            className="toast-mark"
            initial={{ scale: reduced ? 1 : 0.45, rotate: reduced ? 0 : -28, opacity: 0 }}
            animate={{ scale: 1, rotate: 0, opacity: 1 }}
            transition={{ type: "spring", duration: 0.58, bounce: 0.42, delay: 0.09 }}
          >
            {rarity ? (
              <img className="toast-rarity" src={RARITY_ICON[rarity]} width={40} height={40} alt="" />
            ) : (
              <Logo size={34} />
            )}
          </motion.span>

          <motion.p
            className="toast-text"
            initial={{ opacity: 0, x: reduced ? 0 : 10 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ duration: 0.3, ease: [0.23, 1, 0.32, 1], delay: 0.18 }}
          >
            {rarity ? (
              <>
                <span className="toast-eyebrow">
                  {RARITY_NAME[rarity]} {payload.kind === "milestone" ? "milestone" : "achievement"}
                </span>
                <strong className="toast-title">{payload.title}</strong>
                <span className="toast-line">{withEmphasis(payload.text, payload.emphasis)}</span>
              </>
            ) : (
              <>
                {payload.text}
                {payload.emphasis && <strong> {payload.emphasis}</strong>}
              </>
            )}
          </motion.p>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
