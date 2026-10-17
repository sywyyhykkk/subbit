import fs from "node:fs";
import assert from "node:assert/strict";
const categories = JSON.parse(
  fs.readFileSync("catalog/categories.json", "utf8"),
);
const ids = new Set();
for (const file of fs
  .readdirSync("catalog/services")
  .filter((f) => f.endsWith(".json"))) {
  const s = JSON.parse(fs.readFileSync("catalog/services/" + file, "utf8"));
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
    s.plans.length && new Set(s.plans.map((p) => p.id)).size === s.plans.length,
    s.id + " plans",
  );
  for (const p of s.plans) {
    assert(
      p.name && ["recurring", "lifetime"].includes(p.kind),
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
        Number.isInteger(price.cycleCount ?? 1) && (price.cycleCount ?? 1) > 0, s.id + " price cycle");
      const cycle = `${price.cycleUnit}:${price.cycleCount ?? 1}`;
      assert(!cycles.has(cycle), s.id + " duplicate price cycle");
      cycles.add(cycle);
    }
  }
}
console.log(
  `Validated ${ids.size} services and ${categories.length} categories.`,
);
