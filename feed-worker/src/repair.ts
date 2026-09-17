export type Repair = [number, number, number, number];

export const WINDOW_SECS = 700;
export const SETTLE_SECS = 180;
export const RECHECK_SECS = 600;
export const REPAIR_ATTEMPTS = 3;
export const RETAINED_HOURS = 48;
export const REPAIR_MAX_AGE = RETAINED_HOURS * 3600;
export const REPAIR_QUEUE_MAX = 64;

export function pushRepair(queue: Repair[], from: number, to: number, notBefore: number, attempts: number): Repair[] {
  const floor = Math.floor(notBefore) - REPAIR_MAX_AGE;
  const lo = Math.max(Math.floor(from), floor);
  const hi = Math.floor(to);
  if (hi < lo || attempts > REPAIR_ATTEMPTS) return queue;
  const out = [...queue, [lo, hi, Math.floor(notBefore), attempts] as Repair];
  out.sort((a, b) => a[0] - b[0]);
  return out.slice(-REPAIR_QUEUE_MAX);
}

export function repairWindow(
  [from, to, , attempts]: Repair,
  now: number,
): { wait: Repair | null; start: number; target: number } {
  const ceiling = now - SETTLE_SECS;
  if (from > ceiling) return { wait: [from, to, now + SETTLE_SECS, attempts], start: 0, target: 0 };
  const target = Math.min(to, ceiling);
  return { wait: null, start: Math.max(from, target - WINDOW_SECS + 1), target };
}

export function repairFollowUps(
  [from, , , attempts]: Repair,
  plan: { start: number; target: number },
  read: { reached: boolean; oldest: number },
  got: { unrated: number; unratedFrom: number; unratedTo: number },
  now: number,
): Repair[] {
  const out: Repair[] = [];
  const { start, target } = plan;
  if (!read.reached) {
    const unread = read.oldest > 0 ? read.oldest : target;
    const progressed = unread < target;
    out.push([start, unread, progressed ? now : now + SETTLE_SECS, progressed ? attempts : attempts + 1]);
  }
  if (start > from) out.push([from, start - 1, now, attempts]);
  if (got.unrated > 0) out.push([got.unratedFrom, got.unratedTo, now + RECHECK_SECS, attempts + 1]);
  return out;
}
