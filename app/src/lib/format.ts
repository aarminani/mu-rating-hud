export const signed = (n: number | null | undefined) =>
  n == null || n === 0 ? "" : n > 0 ? `+${n}` : `−${Math.abs(n)}`;

export const signedOrZero = (n: number) => (n === 0 ? "±0" : signed(n));

export const shortDate = (unix: number | null) =>
  unix == null
    ? ""
    : new Date(unix * 1000).toLocaleDateString(undefined, {
        month: "numeric",
        day: "numeric",
        year: "2-digit",
      });

export function ago(unix: number, now = Date.now() / 1000): string {
  const s = Math.max(0, now - unix);
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.floor(s / 60)} min ago`;
  if (s < 86400) return `${Math.floor(s / 3600)} h ago`;
  return `${Math.floor(s / 86400)} d ago`;
}

const VOWELS = new Set(["A", "E", "I", "O", "U", "Y"]);

export function monogram(text: string | null | undefined): string {
  const words = (text ?? "").trim().split(/[\s_\-.]+/).filter(Boolean);
  if (words.length === 0) return "?";
  if (words.length >= 2) return (words[0][0] + words[1][0]).toUpperCase();
  const w = words[0].toUpperCase();
  const next = [...w.slice(1)].find((c) => /[A-Z]/.test(c) && !VOWELS.has(c));
  return next ? w[0] + next : w.slice(0, 2);
}

export function grouped<T extends { group: string }>(rs: T[]): [string, T[]][] {
  const out: [string, T[]][] = [];
  for (const r of rs) {
    const last = out[out.length - 1];
    if (last && last[0] === r.group) last[1].push(r);
    else out.push([r.group, [r]]);
  }
  return out;
}

export function remembered<T extends string>(key: string, fallback: T, allowed: readonly T[]): T {
  try {
    const v = localStorage.getItem(key);
    return v && (allowed as readonly string[]).includes(v) ? (v as T) : fallback;
  } catch {
    return fallback;
  }
}

export function remember(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
  }
}
