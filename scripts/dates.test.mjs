import assert from "node:assert/strict";
import test from "node:test";
import { nextCycle, parseDate } from "../src/dates.ts";

test("monthly expiry clamps month ends and preserves the start-date anchor", () => {
  assert.equal(nextCycle("2026-01-31", "month", 1), "2026-02-28");
  assert.equal(nextCycle("2026-01-31", "month", 2), "2026-03-31");
  assert.equal(nextCycle("2028-01-31", "month", 1), "2028-02-29");
  assert.equal(nextCycle("2026-10-01", "month", 1), "2026-11-01");
});
test("annual, weekly and daily cycles use calendar dates", () => {
  assert.equal(nextCycle("2028-02-29", "year", 1), "2029-02-28");
  assert.equal(nextCycle("2026-12-25", "week", 2), "2027-01-08");
  assert.equal(nextCycle("2026-12-31", "day", 1), "2027-01-01");
});
test("invalid dates and incomplete cycle input do not produce an expiry", () => {
  for (const date of ["", "2026-02-30", "2026-13-01", "26-01-01", "1899-01-01"])
    assert.equal(parseDate(date), null);
  for (const count of [0, 1.5, undefined, 121]) assert.equal(nextCycle("2026-01-01", "month", count), null);
  assert.equal(nextCycle("9999-12-01", "month", 1), null);
});
