const GOD_OF_DESTRUCTION = 29;

export interface AuditRecord {
  battle_at: number;
  battle_id?: string;
  p1_rank?: number | null;
  p2_rank?: number | null;
  p1_rating_before?: number | null;
  p2_rating_before?: number | null;
}

export interface Audit {
  at: number;
  from: number;
  to: number;
  wavu: number;
  found: number;
  missing: number;
  blank: number;
  duplicates: number;
  examples: string[];
  ok: boolean;
}

const god = (r: AuditRecord) => (r.p1_rank ?? 0) >= GOD_OF_DESTRUCTION || (r.p2_rank ?? 0) >= GOD_OF_DESTRUCTION;
const rated = (r: AuditRecord) => r.p1_rating_before != null && r.p2_rating_before != null;
const inWindow = (r: AuditRecord, from: number, to: number) => r.battle_at >= from && r.battle_at <= to;

export function compare(wavu: AuditRecord[], feed: AuditRecord[], from: number, to: number, at: number): Audit {
  const want = new Map<string, AuditRecord>();
  for (const r of wavu) {
    if (r.battle_id && god(r) && inWindow(r, from, to)) want.set(r.battle_id, r);
  }
  const have = new Map<string, AuditRecord>();
  let seen = 0;
  for (const r of feed) {
    if (!r.battle_id || !inWindow(r, from, to)) continue;
    seen++;
    have.set(r.battle_id, r);
  }
  const missing: string[] = [];
  for (const id of want.keys()) if (!have.has(id)) missing.push(id);
  let blank = 0;
  for (const r of have.values()) if (god(r) && !rated(r)) blank++;

  return {
    at,
    from,
    to,
    wavu: want.size,
    found: want.size - missing.length,
    missing: missing.length,
    blank,
    duplicates: seen - have.size,
    examples: missing.slice(0, 5),
    ok: missing.length === 0 && blank === 0,
  };
}

export const AUDIT_KEEP = 30;

export function withAudit(previous: Audit[], next: Audit): Audit[] {
  return [...previous, next].slice(-AUDIT_KEEP);
}
