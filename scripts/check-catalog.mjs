import fs from "node:fs";
import assert from "node:assert/strict";
const categories = JSON.parse(
  fs.readFileSync("catalog/categories.json", "utf8"),
);
const validId = (id) => typeof id === "string" && /^[a-z0-9-]{1,100}$/.test(id);
assert(categories.length > 0 && categories.length <= 200 && categories.some((c) => c.id === "other"), "Categories must include other");
assert(new Set(categories.map((c) => c.id)).size === categories.length, "Duplicate categories");
for (const c of categories) assert(validId(c.id) && [c.zh, c.en, c.icon].every((v) => typeof v === "string" && v.trim() && v.length <= 2000), "Category metadata");
const ids = new Set();
for (const file of fs
  .readdirSync("catalog/services")
  .filter((f) => f.endsWith(".json"))) {
  const s = JSON.parse(fs.readFileSync("catalog/services/" + file, "utf8"));
  assert(validId(s.id), "Invalid service ID");
  assert(Array.isArray(s.aliases) && s.aliases.length <= 100 && s.aliases.every((a) => typeof a === "string" && a.length <= 200), s.id + " aliases");
  assert(!ids.has(s.id), `Duplicate ${s.id}`);
  ids.add(s.id);
  assert(
    categories.some((c) => c.id === s.category),
    s.id + " category",
  );
  assert(
    s.id + ".json" === file && s.name && s.source.startsWith("https://"),
    s.id + " metadata",
  );
  assert(fs.existsSync("." + s.icon), s.id + " icon");
  assert(
    s.plans.length && s.plans.length <= 200 && new Set(s.plans.map((p) => p.id)).size === s.plans.length,
    s.id + " plans",
  );
  for (const p of s.plans) {
    assert(
      validId(p.id) && p.name && typeof p.retired === "boolean" && ["recurring", "lifetime"].includes(p.kind) &&
        (!p.cycleUnit || ["day", "week", "month", "year"].includes(p.cycleUnit)),
      s.id + " plan",
    );
    if (!p.prices) continue;
    assert(p.prices.length && p.priceSource?.startsWith("https://") &&
      /^\d{4}-\d{2}-\d{2}$/.test(p.priceReviewedAt), s.id + " price provenance");
    const cycles = new Set();
    for (const price of p.prices) {
      assert(/^\d+(\.\d{1,12})?$/.test(price.amountUsd) &&
        Number.isFinite(Number(price.amountUsd)) && Number(price.amountUsd) > 0, s.id + " USD price");
      assert(p.kind === "lifetime" ? !price.cycleUnit && price.cycleCount == null :
        ["day", "week", "month", "year"].includes(price.cycleUnit) &&
        Number.isInteger(price.cycleCount ?? 1) && (price.cycleCount ?? 1) > 0 && (price.cycleCount ?? 1) <= 120, s.id + " price cycle");
      const cycle = `${price.cycleUnit}:${price.cycleCount ?? 1}`;
      assert(!cycles.has(cycle), s.id + " duplicate price cycle");
      cycles.add(cycle);
    }
  }
}
assert(ids.size > 0 && ids.size <= 5000, "Service count");
console.log(
  `Validated ${ids.size} services and ${categories.length} categories.`,
);
