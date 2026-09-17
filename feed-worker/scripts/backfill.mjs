import { execFileSync } from "node:child_process";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const BASE = "https://mr.tekkenresourcehub.com";
const WAVU = "https://wank.wavu.wiki/api/replays";
const UA = "MuRatingFeed-backfill/0.1 (+https://tekkenresourcehub.com; Tekken 8 MR badge)";
const HORIZON = 8 * 3600;
const WINDOW = 700;
const GOD = 29;
const DRY = process.argv.includes("--dry");
const arg = (name) => {
  const hit = process.argv.find((a) => a.startsWith(`--${name}=`));
  return hit ? Number(hit.slice(name.length + 3)) : null;
};
const FROM = arg("from");
const TO = arg("to");
const KEEP = ["battle_at", "battle_id", "winner",
  "p1_polaris_id", "p1_chara_id", "p1_name", "p1_power", "p1_rank", "p1_rating_before", "p1_rating_change", "p1_rounds", "p1_region_id",
  "p2_polaris_id", "p2_chara_id", "p2_name", "p2_power", "p2_rank", "p2_rating_before", "p2_rating_change", "p2_rounds", "p2_region_id"];

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const trim = (r) => Object.fromEntries(KEEP.filter((k) => r[k] !== undefined && r[k] !== null).map((k) => [k, r[k]]));
const god = (r) => Number(r.p1_rank ?? 0) >= GOD || Number(r.p2_rank ?? 0) >= GOD;
const tmp = mkdtempSync(join(tmpdir(), "mrfeed-backfill-"));

function wrangler(args) {
  return execFileSync(process.platform === "win32" ? "npx.cmd" : "npx", ["--no-install", "wrangler", ...args], {
    encoding: "utf8", stdio: ["ignore", "pipe", "pipe"], shell: process.platform === "win32",
  });
}

function getRemote(key) {
  const f = join(tmp, key.replaceAll("/", "_"));
  wrangler(["r2", "object", "get", `murating-feed/${key}`, "--remote", "--file", f]);
  return f;
}

function putRemote(key, body, cacheControl) {
  const f = join(tmp, key.replaceAll("/", "_"));
  writeFileSync(f, body);
  if (DRY) return;
  wrangler(["r2", "object", "put", `murating-feed/${key}`, "--remote", "--file", f,
    "--content-type", "application/json", "--cache-control", `"${cacheControl}"`]);
}

const latest = await (await fetch(`${BASE}/feed/latest.json`, { headers: { "Cache-Control": "no-cache" } })).json();
const firstMinute = Math.min(...latest.minutes);
const now = Math.floor(Date.now() / 1000);
const cutoff =
  TO ??
  Math.min(...(await (await fetch(`${BASE}/feed/min/${firstMinute}.json`)).json()).map((r) => r.battle_at));
const start = FROM ?? Math.floor((now - HORIZON) / 3600) * 3600;
if (!(cutoff > start)) {
  console.error(`nothing to do: range ends (${cutoff}) at or before it starts (${start})`);
  process.exit(1);
}
console.log(`worker's first minute ${firstMinute}, earliest battle ${cutoff} (${new Date(cutoff * 1000).toISOString()})`);
console.log(`backfilling ${new Date(start * 1000).toISOString()} .. cutoff`);

const CACHE = join(import.meta.dirname, "backfill-cache.json");
const fs = await import("node:fs");
const byId = new Map();
let requests = 0;
const cached = fs.existsSync(CACHE) ? JSON.parse(fs.readFileSync(CACHE, "utf8")) : null;
if (cached && cached.cutoff === cutoff && cached.start === start) {
  for (const r of cached.records) byId.set(r.battle_id ?? `${r.battle_at}|${r.p1_name}`, r);
  console.log(`reusing ${byId.size} records cached from an earlier run`);
}
const haveCache = byId.size > 0;
for (let end = Math.ceil(cutoff / WINDOW) * WINDOW; !haveCache && end > start; end -= WINDOW) {
  if (requests > 0) await sleep(1100);
  requests++;
  const res = await fetch(`${WAVU}?before=${end}`, { headers: { "User-Agent": UA } });
  if (!res.ok) throw new Error(`wavu ${res.status} at before=${end}`);
  const recs = await res.json();
  let kept = 0;
  for (const r of recs) {
    if (r.battle_at >= cutoff || r.battle_at < start || !god(r)) continue;
    const id = r.battle_id ?? `${r.battle_at}|${r.p1_name}|${r.p2_name}`;
    if (!byId.has(id)) { byId.set(id, trim(r)); kept++; }
  }
  process.stdout.write(`window ${end}: ${recs.length} records, ${kept} kept (${requests} requests)\n`);
}

if (requests > 0) {
  fs.writeFileSync(CACHE, JSON.stringify({ cutoff, start, records: [...byId.values()] }));
}

const partialHour = Math.floor(cutoff / 3600);
const extraMinute = firstMinute - 1;
const hours = new Map();
const partial = [];
for (const r of byId.values()) {
  const h = Math.floor(r.battle_at / 3600);
  if (h === partialHour && Math.floor(extraMinute / 60) === partialHour) partial.push(r);
  else { if (!hours.has(h)) hours.set(h, []); hours.get(h).push(r); }
}
const byNewest = (a, b) => b.battle_at - a.battle_at;
const hourList = [...hours.keys()].sort((a, b) => a - b);
for (const h of hourList) {
  const list = hours.get(h).sort(byNewest);
  putRemote(`feed/hour/${h}.json`, JSON.stringify(list), "public, max-age=86400, immutable");
  console.log(`hour ${h} (${new Date(h * 3600000).toISOString()}): ${list.length} records`);
}
if (partial.length) {
  const rolledUp = latest.hours.includes(partialHour);
  if (rolledUp && !DRY) {
    const existing = JSON.parse(fs.readFileSync(getRemote(`feed/hour/${partialHour}.json`), "utf8"));
    const ids = new Set(existing.map((r) => r.battle_id));
    const merged = [...existing, ...partial.filter((r) => !ids.has(r.battle_id))].sort(byNewest);
    putRemote(`feed/hour/${partialHour}.json`, JSON.stringify(merged), "public, max-age=86400, immutable");
    console.log(`hour ${partialHour}: merged ${partial.length} backfilled into ${existing.length} rolled-up records`);
  } else {
    putRemote(`feed/min/${extraMinute}.json`, JSON.stringify(partial.sort(byNewest)), "public, max-age=86400, immutable");
    console.log(`minute ${extraMinute}: ${partial.length} records (partial hour ${partialHour})`);
  }
}

if (!DRY) {
  const sec = new Date().getUTCSeconds();
  if (sec < 10 || sec > 45) await sleep(((sec < 10 ? 12 : 72) - sec) * 1000);
  const current = JSON.parse(await import("node:fs").then((fs) => fs.readFileSync(getRemote("feed/latest.json"), "utf8")));
  current.hours = [...new Set([...current.hours, ...hourList])].sort((a, b) => a - b).slice(-26);
  if (partial.length) current.minutes = [...new Set([extraMinute, ...current.minutes])].sort((a, b) => a - b).slice(-120);
  putRemote("feed/latest.json", JSON.stringify(current), "public, max-age=15");
  console.log(`latest.json: hours ${current.hours.join(",")}; minutes ${current.minutes.length}`);
}
console.log(`done: ${requests} Wavu requests, ${byId.size} records${DRY ? " (dry run, nothing uploaded)" : ""}`);
