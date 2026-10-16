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
  for (const p of s.plans)
    assert(
      p.name && ["recurring", "lifetime"].includes(p.kind),
      s.id + " plan",
    );
}
console.log(
  `Validated ${ids.size} services and ${categories.length} categories.`,
);
