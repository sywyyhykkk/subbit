import assert from "node:assert/strict";
import test from "node:test";
import { presetQuote } from "../src/pricing.ts";

const plan = { kind: "recurring", prices: [
  { amountUsd: "20.00", cycleUnit: "month" },
  { amountUsd: "200.00", cycleUnit: "year" },
] };
const record = { kind: "recurring", cycleUnit: "month", cycleCount: 1, currency: "USD", fixedUsdRate: null };
const db = { rates: { CNY: { usd: "0.125" } }, settings: { fixedRates: {} } };

test("USD needs no rate; CNY divides by USD per currency unit", () => {
  assert.equal(presetQuote(plan, record).amount, "20.00");
  assert.equal(presetQuote(plan, { ...record, currency: "CNY" }, db).amount, "160.00");
});
test("record and global fixed rates take priority over cached rates", () => {
  const fixed = { ...db, settings: { fixedRates: { CNY: "0.2" } } };
  assert.equal(presetQuote(plan, { ...record, currency: "CNY" }, fixed).amount, "100.00");
  assert.equal(presetQuote(plan, { ...record, currency: "CNY", fixedUsdRate: "0.25" }, fixed).amount, "80.00");
});
test("missing or invalid exchange rates never reuse the USD number", () => {
  for (const usd of [undefined, "0", "-1", "NaN", "Infinity"]) {
    const database = { rates: { CNY: { usd } }, settings: { fixedRates: {} } };
    assert.equal(presetQuote(plan, { ...record, currency: "CNY" }, database).status, "missing-rate");
  }
});
test("annual billing uses the annual charge; unsupported cycles are not invented", () => {
  assert.equal(presetQuote(plan, { ...record, cycleUnit: "year" }).amount, "200.00");
  assert.equal(presetQuote(plan, { ...record, cycleUnit: "week" }).status, "unpriced");
  assert.equal(presetQuote(plan, { ...record, cycleCount: 2 }).status, "unpriced");
  assert.equal(presetQuote(undefined, record).status, "unpriced");
  assert.equal(presetQuote(plan, { ...record, kind: "lifetime" }).status, "unpriced");
});
test("lifetime quotes do not acquire a recurring cycle", () => {
  const lifetime = { kind: "lifetime", prices: [{ amountUsd: "120.00" }] };
  assert.equal(presetQuote(lifetime, { ...record, kind: "lifetime" }).amount, "120.00");
});
test("small crypto amounts retain precision and JPY uses whole yen", () => {
  const rates = { rates: { BTC: { usd: "100000" }, JPY: { usd: "0.0067" } }, settings: { fixedRates: {} } };
  assert.equal(presetQuote(plan, { ...record, currency: "BTC" }, rates).amount, "0.00020000");
  assert.equal(presetQuote(plan, { ...record, currency: "JPY" }, rates).amount, "2985");
});
