import assert from "node:assert/strict";
import test from "node:test";
import { localTimestamp, reimbursementStatus } from "../src/display.ts";

test("timestamp uses local calendar fields while date-only quotes stay dates", () => {
  const date = new Date(2026, 9, 17, 20, 5);
  assert.equal(localTimestamp(date.toISOString()), "2026-10-17 20:05");
  assert.equal(localTimestamp("2026-10-17"), "2026-10-17");
  assert.equal(localTimestamp("invalid"), "—");
});
test("reimbursement labels preserve decimal precision and distinguish free assets", () => {
  const status = (reimbursement, reimbursementValue, amount = "20") => reimbursementStatus({ reimbursement, reimbursementValue, amount });
  assert.equal(status("none", "0", "0"), "none");
  assert.equal(status("full", "0"), "full");
  assert.equal(status("percent", "100.00"), "full");
  assert.equal(status("fixed", "020.000"), "full");
  assert.equal(status("fixed", "999999999999.000000000001", "999999999999.000000000002"), "partial");
  assert.equal(status("percent", "0.000"), "none");
});
