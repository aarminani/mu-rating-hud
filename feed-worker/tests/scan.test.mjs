import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import test from "node:test";
import assert from "node:assert/strict";

import { jsonArray, readPrefix, selectRange, selectRated } from "../src/scan.ts";

const GOD = 29;
const bytes = new Uint8Array(await readFile(fileURLToPath(new URL("./fixtures/window.json", import.meta.url))));
const truth = JSON.parse(new TextDecoder().decode(bytes));
const god = (r) => (r.p1_rank ?? 0) >= GOD || (r.p2_rank ?? 0) >= GOD;
const rated = (r) => r.p1_rating_before != null && r.p2_rating_before != null;
const times = truth.map((r) => r.battle_at);
const NEWEST = Math.max(...times);
const OLDEST = Math.min(...times);

function stream() {
  return new Response(bytes.slice()).body;
}

const wholeBody = async () => (await readPrefix(stream(), 0, 8 * 1024 * 1024, 64 * 1024)).body;

test("the fixture still has the shape these tests need", () => {
  assert.ok(truth.length > 50, "enough records to bisect");
  assert.ok(truth.some(god) && truth.some((r) => !god(r)), "both sides of the GoD gate");
  assert.ok(truth.some((r) => !rated(r)), "an unrated band to hold");
  assert.ok(
    truth.filter((r) => !rated(r)).every((r) => r.battle_at >= Math.max(...truth.filter(rated).map((x) => x.battle_at))),
    "the unrated records are the newest ones, which is the assumption the band search rests on",
  );
});

test("readPrefix reads the whole body when nothing is older than the cursor", async () => {
  const p = await readPrefix(stream(), 0, 8 * 1024 * 1024, 64 * 1024);
  assert.equal(p.newest, NEWEST);
  assert.equal(p.oldest, OLDEST);
  assert.equal(p.reached, true);
  assert.equal(JSON.parse(new TextDecoder().decode(jsonArray([p.body]))).length, truth.length);
});

test("readPrefix stops at the first record older than the cursor", async () => {
  const cut = times[Math.floor(times.length / 2)];
  const p = await readPrefix(stream(), cut, 8 * 1024 * 1024, 64 * 1024);
  const got = JSON.parse(new TextDecoder().decode(jsonArray([p.body])));
  assert.equal(p.reached, true);
  assert.ok(got.every((r) => r.battle_at >= cut), "nothing older than the cursor is kept");
  assert.deepEqual(
    got.map((r) => r.battle_id).sort(),
    truth.filter((r) => r.battle_at >= cut).map((r) => r.battle_id).sort(),
  );
});

test("a byte ceiling truncates to the newest part and says so", async () => {
  const p = await readPrefix(stream(), 0, 8 * 1024, 4 * 1024);
  assert.equal(p.reached, false, "the read never got back to the cursor");
  assert.equal(p.newest, NEWEST);
  assert.ok(p.oldest > OLDEST, "it stopped short of the window's oldest record");
  const got = JSON.parse(new TextDecoder().decode(jsonArray([p.body])));
  assert.ok(got.length > 0 && got.length < truth.length);
  assert.equal(
    p.oldest,
    Math.min(...got.map((r) => r.battle_at)),
    "`oldest` is where a repair has to pick up from, so it must be the oldest record actually read",
  );
});

test("selectRated holds the unrated band and publishes the rated GoD+ records under it", async () => {
  const body = await wholeBody();
  const s = selectRated(body, 0, 0);
  const bandStart = Math.min(...truth.filter((r) => !rated(r)).map((r) => r.battle_at));
  assert.equal(s.heldFrom, bandStart, "the band's oldest second is where the cursor stops");

  const got = JSON.parse(new TextDecoder().decode(jsonArray(s.pieces)));
  assert.deepEqual(
    got.map((r) => r.battle_id).sort(),
    truth.filter((r) => god(r) && r.battle_at < bandStart).map((r) => r.battle_id).sort(),
  );
  assert.ok(got.every(god), "nothing below God of Destruction is published");
  assert.equal(s.forcedFrom, 0, "nothing was forced");
  assert.equal(s.forcedTo, 0);
  assert.equal(s.parsed, 0, "every record was read from the bytes, none needed a JSON.parse");
});

test("selectRated forces the band once it is too old, and reports the band it forced", async () => {
  const body = await wholeBody();
  const s = selectRated(body, 0, NEWEST + 1);
  assert.equal(s.heldFrom, 0, "nothing is held when everything can be forced");

  const unratedGod = truth.filter((r) => !rated(r) && god(r));
  const got = JSON.parse(new TextDecoder().decode(jsonArray(s.pieces)));
  assert.deepEqual(
    got.map((r) => r.battle_id).sort(),
    truth.filter(god).map((r) => r.battle_id).sort(),
    "every GoD+ record is published, rated or not",
  );
  assert.equal(s.forcedFrom, Math.min(...unratedGod.map((r) => r.battle_at)));
  assert.equal(s.forcedTo, Math.max(...unratedGod.map((r) => r.battle_at)));
});

test("selectRated publishes nothing older than `from`", async () => {
  const body = await wholeBody();
  const cut = times[Math.floor(times.length / 2)];
  const s = selectRated(body, cut, 0);
  const got = JSON.parse(new TextDecoder().decode(jsonArray(s.pieces)));
  assert.ok(got.every((r) => r.battle_at >= cut));
});

test("selectRange takes exactly the GoD+ records inside an inclusive range", async () => {
  const body = await wholeBody();
  const lo = OLDEST + 5;
  const hi = OLDEST + 10;
  const r = selectRange(body, lo, hi);
  const want = truth.filter((x) => god(x) && x.battle_at >= lo && x.battle_at <= hi);
  const got = JSON.parse(new TextDecoder().decode(jsonArray(r.pieces)));
  assert.equal(r.kept, want.length);
  assert.deepEqual(got.map((x) => x.battle_id).sort(), want.map((x) => x.battle_id).sort());
  assert.ok(got.every((x) => x.battle_at >= lo && x.battle_at <= hi), "both ends are IN the range");
});

test("selectRange over a single second is a real range, not an empty one", async () => {
  const body = await wholeBody();
  const second = truth.find((r) => god(r)).battle_at;
  const r = selectRange(body, second, second);
  const want = truth.filter((x) => god(x) && x.battle_at === second);
  assert.ok(want.length > 0, "the fixture has a GoD+ record at that second");
  assert.equal(r.kept, want.length);
});

test("selectRange reports the records Wavu has still not rated", async () => {
  const body = await wholeBody();
  const r = selectRange(body, OLDEST, NEWEST);
  const unratedGod = truth.filter((x) => god(x) && !rated(x));
  assert.equal(r.unrated, unratedGod.length);
  assert.equal(r.unratedFrom, Math.min(...unratedGod.map((x) => x.battle_at)));
  assert.equal(r.unratedTo, Math.max(...unratedGod.map((x) => x.battle_at)));
});

test("an inverted or empty range keeps nothing", async () => {
  const body = await wholeBody();
  assert.equal(selectRange(body, NEWEST, OLDEST).kept, 0);
  assert.equal(selectRange(body, NEWEST + 1000, NEWEST + 2000).kept, 0);
});

test("an empty body is handled everywhere rather than thrown on", async () => {
  const empty = new Uint8Array(0);
  const s = selectRated(empty, 0, 0);
  assert.equal(s.kept, 0);
  assert.equal(s.heldFrom, 0);
  assert.equal(s.newest, 0);
  assert.equal(selectRange(empty, 0, 9e9).kept, 0);
  assert.equal(new TextDecoder().decode(jsonArray([])), "[]");
});
