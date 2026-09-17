import test from "node:test";
import assert from "node:assert/strict";

import { repairWindow, repairFollowUps } from "../src/repair.ts";

const SETTLE = 180;
const WINDOW = 700;
const RECHECK = 600;
const NOW = 1_800_000_000;
const CEILING = NOW - SETTLE;

const whole = { reached: true, oldest: 0 };
const nothingUnrated = { unrated: 0, unratedFrom: 0, unratedTo: 0 };

test("a range that has settled is read newest window first", () => {
  const plan = repairWindow([CEILING - 5000, CEILING - 1000, 0, 0], NOW);
  assert.equal(plan.wait, null);
  assert.equal(plan.target, CEILING - 1000, "the newest end of the range");
  assert.equal(plan.start, CEILING - 1000 - WINDOW + 1, "as far back as one Wavu window reaches");
});

test("a range newer than the settle ceiling waits, and is never dropped", () => {
  const entry = [NOW - 60, NOW - 10, 0, 1];
  const plan = repairWindow(entry, NOW);
  assert.deepEqual(plan.wait, [NOW - 60, NOW - 10, NOW + SETTLE, 1], "same range, later, same attempts");
});

test("a range straddling the ceiling reads the settled part now", () => {
  const plan = repairWindow([CEILING - 100, NOW - 10, 0, 0], NOW);
  assert.equal(plan.wait, null);
  assert.equal(plan.target, CEILING, "never reads into the part Wavu is still writing");
});

test("a one-second range is read, not skipped", () => {
  const second = CEILING - 900;
  const plan = repairWindow([second, second, 0, 1], NOW);
  assert.equal(plan.wait, null);
  assert.equal(plan.target, second);
  assert.equal(plan.start, second, "start and target are the same second");
  assert.deepEqual(repairFollowUps([second, second, 0, 1], plan, whole, nothingUnrated, NOW), [], "and it is done");
});

test("a finished read of a range wider than one window queues the rest of the range", () => {
  const entry = [CEILING - 5000, CEILING - 1000, 0, 0];
  const plan = repairWindow(entry, NOW);
  const again = repairFollowUps(entry, plan, whole, nothingUnrated, NOW);
  assert.equal(again.length, 1);
  assert.deepEqual(again[0], [CEILING - 5000, plan.start - 1, NOW, 0], "everything under the window just read");
});

test("a finished read of a range inside one window leaves nothing behind", () => {
  const entry = [CEILING - 300, CEILING - 100, 0, 0];
  const plan = repairWindow(entry, NOW);
  assert.deepEqual(repairFollowUps(entry, plan, whole, nothingUnrated, NOW), []);
});

test("a ceiling-stopped read queues the part it never reached", () => {
  const entry = [CEILING - 5000, CEILING - 1000, 0, 0];
  const plan = repairWindow(entry, NOW);
  const stopped = { reached: false, oldest: plan.target - 250 };
  const again = repairFollowUps(entry, plan, stopped, nothingUnrated, NOW);

  const unread = again.find((r) => r[0] === plan.start);
  assert.ok(unread, "the unread part of the window is back on the queue");
  assert.deepEqual(unread, [plan.start, plan.target - 250, NOW, 0]);
  assert.equal(unread[1], stopped.oldest);
  assert.ok(
    again.some((r) => r[0] === entry[0] && r[1] === plan.start - 1),
    "and the rest of the range below the window is still queued too",
  );
});

test("a ceiling-stopped read that made no progress waits and spends an attempt", () => {
  const entry = [CEILING - 5000, CEILING - 1000, 0, 0];
  const plan = repairWindow(entry, NOW);
  const noProgress = { reached: false, oldest: plan.target };
  const again = repairFollowUps(entry, plan, noProgress, nothingUnrated, NOW);
  const stuck = again.find((r) => r[0] === plan.start);
  assert.deepEqual(stuck, [plan.start, plan.target, NOW + SETTLE, 1], "later, and one attempt down");
});

test("a read that returned no complete record at all keeps the whole window", () => {
  const entry = [CEILING - 5000, CEILING - 1000, 0, 0];
  const plan = repairWindow(entry, NOW);
  const nothing = { reached: false, oldest: 0 };
  const again = repairFollowUps(entry, plan, nothing, nothingUnrated, NOW);
  assert.deepEqual(again[0], [plan.start, plan.target, NOW + SETTLE, 1]);
});

test("records Wavu still has not rated come back once more, later", () => {
  const entry = [CEILING - 300, CEILING - 100, 0, 1];
  const plan = repairWindow(entry, NOW);
  const band = { unrated: 3, unratedFrom: CEILING - 250, unratedTo: CEILING - 240 };
  const again = repairFollowUps(entry, plan, whole, band, NOW);
  assert.deepEqual(again, [[CEILING - 250, CEILING - 240, NOW + RECHECK, 2]], "one more attempt against it");
});

test("a ceiling stop and an unrated band are both queued, not one or the other", () => {
  const entry = [CEILING - 5000, CEILING - 1000, 0, 0];
  const plan = repairWindow(entry, NOW);
  const again = repairFollowUps(
    entry,
    plan,
    { reached: false, oldest: plan.target - 250 },
    { unrated: 2, unratedFrom: plan.target - 20, unratedTo: plan.target - 10 },
    NOW,
  );
  assert.equal(again.length, 3, "unread window part, rest of the range, and the unrated band");
});
