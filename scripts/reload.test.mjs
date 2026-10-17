import assert from "node:assert/strict";
import test from "node:test";
import { createReloadQueue } from "../src/reload.ts";

test("simultaneous focus, timer and data events share one reload", async () => {
  const calls = [];
  const reload = createReloadQueue(async (initial) => { calls.push(initial); });
  const first = reload();
  assert.equal(reload(true), first);
  assert.equal(reload(), first);
  await first;
  assert.deepEqual(calls, [true]);
});

test("requests during a reload recheck the latest selection without overlap", async () => {
  const gate = Promise.withResolvers();
  const calls = [];
  let selection = "CNY", active = 0;
  const reload = createReloadQueue(async (initial) => {
    assert.equal(++active, 1);
    calls.push([selection, initial]);
    if (calls.length === 1) await gate.promise;
    active--;
  });
  const first = reload();
  await Promise.resolve();
  selection = "USD";
  assert.equal(reload(), first);
  assert.equal(reload(true), first);
  gate.resolve();
  await first;
  assert.deepEqual(calls, [["CNY", false], ["USD", true]]);
  assert.equal(active, 0);
});

test("a failed reload does not prevent the next attempt", async () => {
  let calls = 0;
  const reload = createReloadQueue(async () => {
    if (++calls === 1) throw new Error("Read failed");
  });
  await assert.rejects(reload(), /Read failed/);
  await reload();
  assert.equal(calls, 2);
});
