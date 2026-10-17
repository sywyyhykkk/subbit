import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

export function buildCatalog(root = "catalog") {
  const categories = JSON.parse(fs.readFileSync(path.join(root, "categories.json"), "utf8"));
  const services = fs.readdirSync(path.join(root, "services")).filter((file) => file.endsWith(".json")).sort()
    .map((file) => JSON.parse(fs.readFileSync(path.join(root, "services", file), "utf8")));
  const icons = {};
  for (const service of services) {
    if (!/^\/catalog\/icons\/[a-z0-9-]+\.(svg|png)$/.test(service.icon)) throw new Error(`Invalid icon: ${service.id}`);
    const bytes = fs.readFileSync(path.join(root, "icons", path.basename(service.icon)));
    if (!bytes.length || bytes.length > 150_000) throw new Error(`Icon exceeds 150 KB: ${service.id}`);
    const mime = service.icon.endsWith(".svg") ? "image/svg+xml" : "image/png";
    if (mime === "image/png" ? !bytes.subarray(0, 8).equals(Buffer.from([137,80,78,71,13,10,26,10])) :
      !bytes.toString("utf8").includes("<svg") || !bytes.toString("utf8").includes("</svg>")) throw new Error(`Invalid icon content: ${service.id}`);
    icons[service.icon] = `data:${mime};base64,${bytes.toString("base64")}`;
  }
  const catalog = { schemaVersion: 1, categories, services, icons };
  if (Buffer.byteLength(JSON.stringify(catalog)) > 7_900_000) throw new Error("Catalog exceeds download limit");
  return catalog;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const output = process.argv[2] ?? "artifacts/catalog.json";
  fs.mkdirSync(path.dirname(output), { recursive: true });
  const catalog = buildCatalog();
  fs.writeFileSync(output, JSON.stringify(catalog));
  console.log(`Packed ${catalog.services.length} services, ${catalog.categories.length} categories and ${Object.keys(catalog.icons).length} icons into ${output}`);
}
