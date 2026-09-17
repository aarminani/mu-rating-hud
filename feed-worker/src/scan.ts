const MARK = [0x7b, 0x22, 0x62, 0x61, 0x74, 0x74, 0x6c, 0x65, 0x5f, 0x61, 0x74, 0x22, 0x3a];
const HEAD = MARK.length + 11;
const OPEN = 0x7b;

export const EMPTY = new Uint8Array(0);

export function recordTime(b: Uint8Array, at: number): number {
  if (at < 0 || at + HEAD > b.length) return -1;
  for (let k = 0; k < MARK.length; k++) if (b[at + k] !== MARK[k]) return -1;
  const p = at + MARK.length;
  let t = 0;
  for (let k = 0; k < 10; k++) {
    const d = b[p + k] - 48;
    if (d < 0 || d > 9) return -1;
    t = t * 10 + d;
  }
  const after = b[p + 10] - 48;
  return after >= 0 && after <= 9 ? -1 : t;
}

export function nextRecord(b: Uint8Array, from: number): number {
  for (let at = b.indexOf(OPEN, from); at >= 0; at = b.indexOf(OPEN, at + 1)) {
    if (recordTime(b, at) >= 0) return at;
  }
  return -1;
}

export function lastRecord(b: Uint8Array, floor: number): number {
  for (let at = b.lastIndexOf(OPEN); at >= floor && at >= 0; at = at > 0 ? b.lastIndexOf(OPEN, at - 1) : -1) {
    if (recordTime(b, at) >= 0) return at;
  }
  return -1;
}

export function firstOlder(b: Uint8Array, from: number, to: number, since: number): number {
  let lo = from;
  let hi = to;
  let best = -1;
  while (hi - lo > 4096) {
    const mid = lo + ((hi - lo) >> 1);
    const at = nextRecord(b, mid);
    if (at < 0 || at >= hi) {
      hi = mid;
    } else if (recordTime(b, at) < since) {
      best = at;
      hi = mid;
    } else {
      lo = at + 1;
    }
  }
  for (let at = nextRecord(b, lo); at >= 0 && at < hi; at = nextRecord(b, at + 1)) {
    if (recordTime(b, at) < since) return at;
  }
  return best;
}

export interface Prefix {
  body: Uint8Array;
  newest: number;
  oldest: number;
  read: number;
  reached: boolean;
}

const MIN_READ = 64 * 1024;

interface MinReader {
  read(view: Uint8Array, options: { min: number }): Promise<ReadableStreamReadResult<Uint8Array>>;
}

export async function readPrefix(stream: ReadableStream<Uint8Array>, since: number, capacity: number): Promise<Prefix> {
  const reader = stream.getReader({ mode: "byob" });
  const size = capacity;
  let buffer = new ArrayBuffer(size);
  let len = 0;
  let first = -1;
  let checked = 0;
  let cut = -1;
  let done = false;
  try {
    while (len < capacity) {
      const into = new Uint8Array(buffer, len, size - len);
      const min = Math.min(MIN_READ, size - len);
      const r = "readAtLeast" in reader ? await reader.readAtLeast(min, into) : await (reader as MinReader).read(into, { min });
      let short = false;
      if (r.value) {
        buffer = r.value.buffer as ArrayBuffer;
        len += r.value.byteLength;
        short = r.value.byteLength < min;
      }
      const view = new Uint8Array(buffer, 0, len);
      if (first < 0) first = nextRecord(view, 0);
      if (first >= 0) {
        checked = Math.max(checked, first);
        const last = lastRecord(view, checked);
        if (last >= 0) {
          if (recordTime(view, last) < since) {
            cut = firstOlder(view, checked, last + 1, since);
            break;
          }
          checked = last + 1;
        }
      }
      if (r.done || short) {
        done = true;
        break;
      }
    }
  } finally {
    await reader.cancel().catch(() => {});
  }

  const view = new Uint8Array(buffer, 0, len);
  if (first < 0) first = nextRecord(view, 0);
  if (first < 0) return { body: EMPTY, newest: 0, oldest: 0, read: len, reached: done || cut >= 0 };

  let end: number;
  if (cut >= 0) {
    end = cut;
  } else if (done) {
    const close = view.lastIndexOf(0x5d);
    end = close > first ? close : len;
  } else {
    const last = lastRecord(view, first + 1);
    end = last > 0 ? last : first;
  }
  while (end > first && (view[end - 1] === 0x2c || view[end - 1] <= 0x20)) end--;
  const body = view.subarray(first, end);
  const last = body.length ? lastRecord(body, 0) : -1;
  return {
    body,
    newest: body.length ? recordTime(view, first) : 0,
    oldest: last >= 0 ? recordTime(body, last) : 0,
    read: len,
    reached: cut >= 0 || done,
  };
}

const RANK_KEY = [0x5f, 0x72, 0x61, 0x6e, 0x6b, 0x22, 0x3a];
const RANK_ANCHOR = 4;
const BEFORE_KEY = [0x2c, 0x22, 0x70, 0x00, 0x5f, 0x72, 0x61, 0x74, 0x69, 0x6e, 0x67, 0x5f, 0x62, 0x65, 0x66, 0x6f, 0x72, 0x65, 0x22, 0x3a];
const GOD_OF_DESTRUCTION = 29;

interface Side {
  rank: number;
  rated: boolean;
  next: number;
}

function side(b: Uint8Array, from: number, to: number, n: number): Side | null {
  let at = b.indexOf(0x6b, from + RANK_ANCHOR);
  for (; at >= 0 && at < to; at = b.indexOf(0x6b, at + 1)) {
    const start = at - RANK_ANCHOR;
    let ok = b[start - 1] === 0x30 + n && b[start - 2] === 0x70 && b[start - 3] === 0x22;
    for (let k = 0; ok && k < RANK_KEY.length; k++) ok = b[start + k] === RANK_KEY[k];
    if (!ok) continue;
    let p = start + RANK_KEY.length;
    let rank = 0;
    if (b[p] === 0x6e) {
      p += 4;
    } else {
      while (p < to && b[p] >= 0x30 && b[p] <= 0x39) rank = rank * 10 + (b[p++] - 0x30);
    }
    for (let k = 0; k < BEFORE_KEY.length; k++) {
      if (b[p + k] !== (k === 3 ? 0x30 + n : BEFORE_KEY[k])) return null;
    }
    const value = p + BEFORE_KEY.length;
    return { rank, rated: b[value] !== 0x6e, next: value + 1 };
  }
  return null;
}

function judger(body: Uint8Array) {
  const state = { parsed: 0 };
  const judge = (at: number, end: number): { rated: boolean; god: boolean } => {
    const s1 = side(body, at, end, 1);
    const s2 = s1 ? side(body, s1.next, end, 2) : null;
    if (s1 && s2) {
      return { rated: s1.rated && s2.rated, god: s1.rank >= GOD_OF_DESTRUCTION || s2.rank >= GOD_OF_DESTRUCTION };
    }
    state.parsed++;
    try {
      let e = end;
      while (e > at && (body[e - 1] === 0x2c || body[e - 1] <= 0x20 || body[e - 1] === 0x5d)) e--;
      const r = JSON.parse(new TextDecoder().decode(body.subarray(at, e)));
      return {
        rated: r.p1_rating_before != null && r.p2_rating_before != null,
        god: (r.p1_rank ?? 0) >= GOD_OF_DESTRUCTION || (r.p2_rank ?? 0) >= GOD_OF_DESTRUCTION,
      };
    } catch {
      return { rated: false, god: false };
    }
  };
  const endOf = (at: number) => {
    const following = nextRecord(body, at + 1);
    return following >= 0 ? following : body.length;
  };
  const piece = (at: number, end: number) => {
    let e = end;
    while (e > at && (body[e - 1] === 0x2c || body[e - 1] <= 0x20)) e--;
    return body.subarray(at, e);
  };
  return { judge, endOf, piece, state };
}

export interface Selected {
  pieces: Uint8Array[];
  bytes: number;
  records: number;
  kept: number;
  newest: number;
  heldFrom: number;
  forcedFrom: number;
  forcedTo: number;
  parsed: number;
}

export function selectRated(body: Uint8Array, from: number, forceBefore: number): Selected {
  const { judge, endOf, state } = judger(body);
  const settled = (at: number) => judge(at, endOf(at)).rated || recordTime(body, at) < forceBefore;

  const first = nextRecord(body, 0);
  let edge = body.length;
  if (first >= 0) {
    if (settled(first)) {
      edge = first;
    } else {
      let lo = first + 1;
      let hi = body.length;
      while (hi - lo > 1024) {
        const mid = lo + ((hi - lo) >> 1);
        const at = nextRecord(body, mid);
        if (at < 0 || at >= hi) {
          hi = mid;
        } else if (settled(at)) {
          edge = at;
          hi = mid;
        } else {
          lo = at + 1;
        }
      }
      for (let at = nextRecord(body, lo); at >= 0 && at < hi; at = nextRecord(body, at + 1)) {
        if (settled(at)) {
          edge = at;
          break;
        }
      }
    }
  }
  let heldFrom = 0;
  if (first >= 0 && edge > first) {
    const lastInBand = lastRecord(body.subarray(0, edge), first);
    heldFrom = recordTime(body, lastInBand >= 0 ? lastInBand : first);
  }

  const starts: number[] = [];
  const times: number[] = [];
  const flags: number[] = [];
  let judged = 0;
  for (let at = edge < body.length ? edge : -1; at >= 0; ) {
    const end = endOf(at);
    const t = recordTime(body, at);
    const { rated, god } = judge(at, end);
    judged++;
    if (!rated && t >= forceBefore && (heldFrom === 0 || t < heldFrom)) heldFrom = t;
    starts.push(at);
    times.push(t);
    flags.push((rated ? 1 : 0) | (god ? 2 : 0));
    at = end < body.length ? end : -1;
  }

  const pieces: Uint8Array[] = [];
  let bytes = 0;
  let forcedFrom = 0;
  let forcedTo = 0;
  for (let i = 0; i < starts.length; i++) {
    if (!(flags[i] & 2) || times[i] < from) continue;
    if (heldFrom !== 0 && times[i] >= heldFrom) continue;
    let e = i + 1 < starts.length ? starts[i + 1] : body.length;
    while (e > starts[i] && (body[e - 1] === 0x2c || body[e - 1] <= 0x20)) e--;
    const piece = body.subarray(starts[i], e);
    pieces.push(piece);
    bytes += piece.length;
    if (!(flags[i] & 1)) {
      forcedFrom = forcedFrom === 0 ? times[i] : Math.min(forcedFrom, times[i]);
      forcedTo = Math.max(forcedTo, times[i]);
    }
  }
  return {
    pieces,
    bytes,
    records: judged,
    kept: pieces.length,
    newest: first >= 0 ? recordTime(body, first) : 0,
    heldFrom,
    forcedFrom,
    forcedTo,
    parsed: state.parsed,
  };
}

export interface Ranged {
  pieces: Uint8Array[];
  bytes: number;
  kept: number;
  unrated: number;
  unratedFrom: number;
  unratedTo: number;
  parsed: number;
}

export function selectRange(body: Uint8Array, from: number, to: number): Ranged {
  const { judge, endOf, piece, state } = judger(body);
  const pieces: Uint8Array[] = [];
  let bytes = 0;
  let unrated = 0;
  let unratedFrom = 0;
  let unratedTo = 0;
  for (let at = nextRecord(body, 0); at >= 0; ) {
    const end = endOf(at);
    const t = recordTime(body, at);
    if (t >= from && t <= to) {
      const { rated, god } = judge(at, end);
      if (god) {
        const p = piece(at, end);
        pieces.push(p);
        bytes += p.length;
        if (!rated) {
          unrated++;
          unratedFrom = unratedFrom === 0 ? t : Math.min(unratedFrom, t);
          unratedTo = Math.max(unratedTo, t);
        }
      }
    }
    at = end < body.length ? end : -1;
  }
  return { pieces, bytes, kept: pieces.length, unrated, unratedFrom, unratedTo, parsed: state.parsed };
}

export function jsonArray(pieces: Uint8Array[]): Uint8Array {
  const kept = pieces.filter((p) => p.length > 0);
  let total = 2 + Math.max(0, kept.length - 1);
  for (const p of kept) total += p.length;
  const out = new Uint8Array(total);
  let o = 0;
  out[o++] = 0x5b;
  kept.forEach((p, i) => {
    if (i) out[o++] = 0x2c;
    out.set(p, o);
    o += p.length;
  });
  out[o] = 0x5d;
  return out;
}
