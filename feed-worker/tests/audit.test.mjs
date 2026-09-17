import test from "node:test";
import assert from "node:assert/strict";

import { compare, withAudit, AUDIT_KEEP } from "../src/audit.ts";

const FROM = 1_800_000_000;
const TO = FROM + 699;
const AT = TO + 4 * 3600;

const battle = (id, over = {}) => ({
  battle_at: FROM + 100,
  battle_id: id,
  p1_rank: 31,
  p2_rank: 30,
  p1_rating_before: 2000,
  p2_rating_before: 1950,
  ...over,
});

test("a feed holding everything Wavu has passes", () => {
  const wavu = [battle("a"), battle("b"), battle("c")];
  const r = compare(wavu, [...wavu], FROM, TO, AT);
  assert.equal(r.wavu, 3);
  assert.equal(r.found, 3);
  assert.equal(r.missing, 0);
  assert.equal(r.ok, true);
});

test("a battle Wavu has and the feed does not is counted, with an example", () => {
  const wavu = [battle("a"), battle("b"), battle("gone")];
  const r = compare(wavu, [battle("a"), battle("b")], FROM, TO, AT);
  assert.equal(r.missing, 1);
  assert.equal(r.found, 2);
  assert.deepEqual(r.examples, ["gone"], "the id, so a bad day can be chased");
  assert.equal(r.ok, false);
});

test("battles below God of Destruction are not expected of the feed", () => {
  const wavu = [battle("god"), battle("low", { p1_rank: 20, p2_rank: 18 })];
  const r = compare(wavu, [battle("god")], FROM, TO, AT);
  assert.equal(r.wavu, 1);
  assert.equal(r.missing, 0);
  assert.equal(r.ok, true);
});

test("a GoD+ side is enough, either side of the battle", () => {
  const wavu = [battle("p2only", { p1_rank: 12, p2_rank: 29 })];
  const r = compare(wavu, [], FROM, TO, AT);
  assert.equal(r.wavu, 1);
  assert.equal(r.missing, 1);
});

test("records outside the window are ignored on both sides", () => {
  const wavu = [battle("in"), battle("before", { battle_at: FROM - 1 }), battle("after", { battle_at: TO + 1 })];
  const feed = [battle("in"), battle("stranger", { battle_at: TO + 5000 })];
  const r = compare(wavu, feed, FROM, TO, AT);
  assert.equal(r.wavu, 1, "only the one in the window is expected");
  assert.equal(r.missing, 0);
  assert.equal(r.duplicates, 0, "the stranger is not counted at all");
});

test("the window includes both of its ends", () => {
  const wavu = [battle("first", { battle_at: FROM }), battle("last", { battle_at: TO })];
  const r = compare(wavu, [], FROM, TO, AT);
  assert.equal(r.wavu, 2);
});

test("a battle published twice is a duplicate, not a second battle", () => {
  const wavu = [battle("a")];
  const r = compare(wavu, [battle("a"), battle("a")], FROM, TO, AT);
  assert.equal(r.found, 1);
  assert.equal(r.duplicates, 1);
  assert.equal(r.ok, true, "duplicates alone do not fail an audit");
});

test("a published record with no rating fails the audit", () => {
  const wavu = [battle("a")];
  const feed = [battle("a", { p1_rating_before: null, p2_rating_before: null })];
  const r = compare(wavu, feed, FROM, TO, AT);
  assert.equal(r.missing, 0, "it IS there");
  assert.equal(r.blank, 1, "but it is useless");
  assert.equal(r.ok, false);
});

test("a record with no battle id cannot be matched and is not counted", () => {
  const r = compare([battle(undefined)], [], FROM, TO, AT);
  assert.equal(r.wavu, 0);
  assert.equal(r.ok, true);
});

test("the history keeps a month and drops the oldest", () => {
  let history = [];
  for (let i = 0; i < AUDIT_KEEP + 5; i++) history = withAudit(history, compare([], [], FROM, TO, AT + i));
  assert.equal(history.length, AUDIT_KEEP);
  assert.equal(history[history.length - 1].at, AT + AUDIT_KEEP + 4, "newest last");
  assert.equal(history[0].at, AT + 5, "and the five oldest are gone");
});
