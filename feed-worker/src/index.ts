import { jsonArray, readPrefix, selectRange, selectRated, type Prefix, type Selected } from "./scan";
import { compare, withAudit, type Audit, type AuditRecord } from "./audit";
import {
  pushRepair,
  repairFollowUps,
  repairWindow,
  RECHECK_SECS,
  RETAINED_HOURS,
  SETTLE_SECS,
  WINDOW_SECS,
  type Repair,
} from "./repair";

export interface Env {
  FEED: R2Bucket;
}

const WAVU = "https://wank.wavu.wiki/api/replays";
const USER_AGENT = "MuRatingFeed/0.2 (+https://tekkenresourcehub.com; Tekken 8 MR badge)";
const KEEP_MINUTES = 120;
const KEEP_HOURS = RETAINED_HOURS;
const AUDIT_CRON = "17 4 * * *";
const AUDIT_AGE_SECS = 4 * 3600;

const COLD_START_SECS = 180;
const PASS_BYTES = 8 * 1024 * 1024;
const REACH_SECS = 3 * 3600;
const STEP_SECS = 600;
const HELD_STEP_SECS = 300;
const LOOKBACK_SECS = 100;
const FORCE_SECS = 900;
const REPAIR_WINDOWS_PER_RUN = 2;
const RETRY_PAUSE_MS = 1000;

interface Latest {
  v: 1;
  minute: number;
  newest_at: number;
  newest_ids: string[];
  minutes: number[];
  hours: number[];
  recheck_at?: number;
  held_at?: number;
  parts?: [number, number, number][];
  decades?: [number, number][];
  repair?: Repair[];
}

interface Piece {
  key: string;
  offset: number;
  length: number;
}

const JSON_IMMUTABLE = { contentType: "application/json", cacheControl: "public, max-age=86400, immutable" };

export default {
  async fetch(): Promise<Response> {
    return new Response("murating-feed is a scheduled Worker. The feed is served from R2 at mr.tekkenresourcehub.com/feed/", {
      status: 410,
      headers: { "content-type": "text/plain; charset=utf-8", "cache-control": "public, max-age=3600" },
    });
  },

  async scheduled(controller: ScheduledController, env: Env): Promise<void> {
    const minute = Math.floor(controller.scheduledTime / 60_000);
    const now = Math.floor(controller.scheduledTime / 1000);

    if (controller.cron === AUDIT_CRON) {
      await runAudit(env, now).catch((e) => console.log(`audit failed: ${e}`));
      return;
    }

    const latest = await readLatest(env);
    const next: Latest = latest ?? { v: 1, minute: 0, newest_at: now - COLD_START_SECS, newest_ids: [], minutes: [], hours: [] };

    const since = Math.max(next.newest_at, now - REACH_SECS);
    if (since > next.newest_at) queueRepair(next, next.newest_at, since, now, 0);
    const waiting = next.held_at !== undefined && next.held_at === since;
    const target = Math.min(now, since + (waiting ? HELD_STEP_SECS : STEP_SECS));
    const read = await readWavu(`${WAVU}?before=${target}`, since - LOOKBACK_SECS);
    const picked = read ? selectRated(read.body, since, now - FORCE_SECS) : null;
    if (read && !read.reached && read.oldest > 0) queueRepair(next, since, read.oldest, now, 0);
    if (picked?.forcedTo) queueRepair(next, picked.forcedFrom, picked.forcedTo, now + RECHECK_SECS, 1);

    next.newest_at = advance(since, target, read, picked, now - SETTLE_SECS);
    next.newest_ids = [];
    delete next.recheck_at;
    if (picked?.heldFrom) next.held_at = next.newest_at;
    else if (read) delete next.held_at;

    let repaired = { pieces: [] as Uint8Array[], log: "" };
    try {
      repaired = await repairPass(env, next, now);
    } catch (e) {
      repaired.log = `repair failed: ${e}`;
    }

    const pieces = [...(picked?.pieces ?? []), ...repaired.pieces];
    if (pieces.length > 0) {
      const file = jsonArray(pieces);
      await env.FEED.put(`feed/min/${minute}.json`, file, { httpMetadata: JSON_IMMUTABLE });
      next.minute = minute;
      next.minutes = [...next.minutes.filter((m) => m !== minute), minute].slice(-KEEP_MINUTES);
      next.parts = [...(next.parts ?? []).filter(([m]) => m !== minute), [minute, 1, file.length - 2]];
    }
    const oldest = next.minutes[0] ?? minute;
    next.parts = (next.parts ?? []).filter(([m]) => m >= oldest);

    let rolled = "";
    try {
      if (rollUpDue(next, minute)) rolled = await rollUp(env, next, minute);
    } catch (e) {
      rolled = `failed: ${e}`;
    }

    await writeLatest(env, next);

    const p = picked;
    console.log(
      `minute ${minute}: read ${read?.read ?? 0} B, ${p?.records ?? 0} records, published ${p?.kept ?? 0} (${p?.bytes ?? 0} B)` +
        `${p?.heldFrom ? `, held from ${p.heldFrom} (${now - p.heldFrom} s old)` : ""}${p?.parsed ? `, ${p.parsed} parsed` : ""}` +
        `${p?.forcedTo ? `, forced ${p.forcedFrom}-${p.forcedTo} unrated` : ""}` +
        `${repaired.log ? `, ${repaired.log}` : ""}${rolled ? `, ${rolled}` : ""}` +
        `, queue ${(next.repair ?? []).length}, newest_at ${next.newest_at}`,
    );
  },
} satisfies ExportedHandler<Env>;

function queueRepair(next: Latest, from: number, to: number, notBefore: number, attempts: number): void {
  next.repair = pushRepair(next.repair ?? [], from, to, notBefore, attempts);
}

async function repairPass(env: Env, next: Latest, now: number): Promise<{ pieces: Uint8Array[]; log: string }> {
  const pieces: Uint8Array[] = [];
  const notes: string[] = [];
  for (let n = 0; n < REPAIR_WINDOWS_PER_RUN; n++) {
    const queue = next.repair ?? [];
    const i = queue.findIndex(([, , notBefore]) => notBefore <= now);
    if (i < 0) break;
    const entry = queue[i];
    const plan = repairWindow(entry, now);
    if (plan.wait) {
      next.repair = queue.map((e, k) => (k === i ? plan.wait! : e));
      continue;
    }
    const { start, target } = plan;
    const read = await readWavu(`${WAVU}?before=${target}`, start);
    if (!read) {
      notes.push(`repair ${start}-${target} read failed`);
      break;
    }
    const got = selectRange(read.body, start, target);
    pieces.push(...got.pieces);
    next.repair = queue.filter((_, k) => k !== i);
    for (const again of repairFollowUps(entry, plan, read, got, now)) {
      queueRepair(next, again[0], again[1], again[2], again[3]);
    }
    notes.push(
      `repaired ${start}-${target}: ${got.kept} records${got.unrated ? `, ${got.unrated} still unrated` : ""}` +
        `${read.reached ? "" : `, ceiling stopped it at ${read.oldest}`}`,
    );
  }
  return { pieces, log: notes.join("; ") };
}

async function runAudit(env: Env, now: number): Promise<void> {
  const to = now - AUDIT_AGE_SECS;
  const from = to - WINDOW_SECS + 1;

  const res = await fetch(`${WAVU}?before=${to}`, {
    headers: { "User-Agent": USER_AGENT, Accept: "application/json" },
  });
  if (!res.ok) {
    console.log(`audit: wavu ${res.status}`);
    await res.body?.cancel();
    return;
  }
  const wavu = (await res.json()) as AuditRecord[];

  const first = Math.floor(from / 3600);
  const feed: AuditRecord[] = [];
  const read: number[] = [];
  for (let h = first; h <= first + 2; h++) {
    const obj = await env.FEED.get(`feed/hour/${h}.json`);
    if (!obj) continue;
    feed.push(...((await obj.json()) as AuditRecord[]));
    read.push(h);
  }
  if (read.length === 0) {
    console.log(`audit: no hour file for ${first}, nothing to compare`);
    return;
  }

  const result = compare(wavu, feed, from, to, now);
  const previous = await env.FEED.get("feed/audit.json");
  const history = previous ? ((await previous.json()) as Audit[]) : [];
  await env.FEED.put("feed/audit.json", JSON.stringify(withAudit(Array.isArray(history) ? history : [], result)), {
    httpMetadata: { contentType: "application/json", cacheControl: "public, max-age=300" },
  });

  console.log(
    `audit ${from}-${to} over hours ${read.join(",")}: wavu ${result.wavu} GoD+, feed ${result.found}, ` +
      `MISSING ${result.missing}, blank ${result.blank}, duplicates ${result.duplicates}` +
      `${result.examples.length ? `, e.g. ${result.examples.join(" ")}` : ""}`,
  );
}

function rollUpDue(next: Latest, minute: number): boolean {
  if (minute % 10 === 2 || minute % 10 === 4) {
    const d = Math.floor(minute / 10) - 1;
    return !next.hours.includes(Math.floor(d / 6)) && !(next.decades ?? []).some(([x]) => x === d);
  }
  if (minute % 60 === 5 || minute % 60 === 7) {
    const h = Math.floor(minute / 60) - 1;
    return !next.hours.includes(h);
  }
  return false;
}

function advance(since: number, target: number, p: Prefix | null, s: Selected | null, settled: number): number {
  if (!p || !s) return since;
  if (!p.reached) console.log(`run [${since}, ${target}] hit the ${PASS_BYTES} B ceiling; the older part is skipped`);
  if (s.heldFrom !== 0) return s.heldFrom;
  if (s.newest < since) return target <= settled ? target : since;
  return Math.max(since, s.newest);
}

async function readWavu(url: string, since: number): Promise<Prefix | null> {
  for (let attempt = 0; attempt < 2; attempt++) {
    if (attempt > 0) await new Promise((r) => setTimeout(r, RETRY_PAUSE_MS));
    try {
      const res = await fetch(url, { headers: { "User-Agent": USER_AGENT, Accept: "application/json", "Accept-Encoding": "identity" } });
      if (!res.ok || !res.body) {
        console.log(`wavu ${res.status} for ${url}${attempt ? " (retry)" : ""}`);
        await res.body?.cancel();
        continue;
      }
      return await readPrefix(res.body, since, PASS_BYTES);
    } catch (e) {
      console.log(`wavu ${url}${attempt ? " (retry)" : ""}: ${e}`);
    }
  }
  return null;
}

async function readLatest(env: Env): Promise<Latest | null> {
  const obj = await env.FEED.get("feed/latest.json");
  if (!obj) return null;
  try {
    const parsed = (await obj.json()) as Latest;
    return parsed?.v === 1 ? parsed : null;
  } catch {
    return null;
  }
}

async function writeLatest(env: Env, latest: Latest): Promise<void> {
  await env.FEED.put("feed/latest.json", JSON.stringify(latest), {
    httpMetadata: { contentType: "application/json", cacheControl: "public, max-age=15" },
  });
}

async function rollUp(env: Env, next: Latest, minute: number): Promise<string> {
  const parts = next.parts ?? [];
  const decades = next.decades ?? [];

  if (minute % 10 === 2 || minute % 10 === 4) {
    const d = Math.floor(minute / 10) - 1;
    const h = Math.floor(d / 6);
    if (next.hours.includes(h) || decades.some(([x]) => x === d)) return "";
    const pieces = minutePieces(parts, d);
    const bytes = pieces.length > 0 ? await join(env, `feed/min/d${d}.part`, pieces, false) : 0;
    next.decades = [...decades, [d, bytes]];
    return `decade ${d}: ${pieces.length} minutes, ${bytes} B`;
  }

  if (minute % 60 === 5 || minute % 60 === 7) {
    const h = Math.floor(minute / 60) - 1;
    if (next.hours.includes(h)) return "";
    const pieces: Piece[] = [];
    let fallback = 0;
    for (let d = h * 6; d < h * 6 + 6; d++) {
      const done = decades.find(([x]) => x === d);
      if (done) {
        if (done[1] > 0) pieces.push({ key: `feed/min/d${d}.part`, offset: 0, length: done[1] });
      } else {
        const own = minutePieces(parts, d);
        fallback += own.length;
        pieces.push(...own);
      }
    }
    if (pieces.length === 0) return "";
    const bytes = await join(env, `feed/hour/${h}.json`, pieces, true);
    next.hours = [...next.hours, h].slice(-KEEP_HOURS);
    const folded = decades.filter(([x]) => Math.floor(x / 6) === h);
    next.decades = decades.filter(([x]) => Math.floor(x / 6) > h);
    const doomed = folded.filter(([, b]) => b > 0).map(([x]) => `feed/min/d${x}.part`);
    if (doomed.length > 0) await env.FEED.delete(doomed).catch(() => {});
    return `hour ${h}: ${pieces.length} pieces (${fallback} from minute files), ${bytes} B`;
  }

  return "";
}

function minutePieces(parts: [number, number, number][], d: number): Piece[] {
  return parts
    .filter(([m, , length]) => Math.floor(m / 10) === d && length > 0)
    .sort((a, b) => a[0] - b[0])
    .map(([m, offset, length]) => ({ key: `feed/min/${m}.json`, offset, length }));
}

const OPEN_BRACKET = new Uint8Array([0x5b]);
const CLOSE_BRACKET = new Uint8Array([0x5d]);
const COMMA = new Uint8Array([0x2c]);

async function join(env: Env, key: string, pieces: Piece[], bracket: boolean): Promise<number> {
  let total = pieces.length - 1 + (bracket ? 2 : 0);
  for (const p of pieces) total += p.length;
  const stream = new FixedLengthStream(total);
  const upload = env.FEED.put(key, stream.readable, {
    httpMetadata: bracket ? JSON_IMMUTABLE : { contentType: "application/octet-stream" },
  });
  upload.catch(() => {});
  const out = stream.writable;
  try {
    if (bracket) await writeBytes(out, OPEN_BRACKET);
    for (let i = 0; i < pieces.length; i++) {
      if (i > 0) await writeBytes(out, COMMA);
      const { key: from, offset, length } = pieces[i];
      const obj = await env.FEED.get(from, { range: { offset, length } });
      if (!obj) throw new Error(`${from} is missing`);
      await obj.body.pipeTo(out, { preventClose: true });
    }
    if (bracket) await writeBytes(out, CLOSE_BRACKET);
    await out.close();
  } catch (e) {
    await out.abort(e).catch(() => {});
    throw e;
  }
  await upload;
  return total;
}

async function writeBytes(out: WritableStream, bytes: Uint8Array): Promise<void> {
  const w = out.getWriter();
  try {
    await w.write(bytes);
  } finally {
    w.releaseLock();
  }
}
