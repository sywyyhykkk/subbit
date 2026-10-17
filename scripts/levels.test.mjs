import assert from "node:assert/strict";
import test from "node:test";
import { defaultLevels, localizeLevelName } from "../src/levels.ts";

test("saved default names follow the active language in either direction", () => {
  const zh = defaultLevels();
  const en = defaultLevels(true);
  for (let i = 0; i < zh.length; i++) {
    assert.equal(localizeLevelName(en[i].name, false), zh[i].name);
    assert.equal(localizeLevelName(zh[i].name, true), en[i].name);
    assert.equal(localizeLevelName(zh[i].name, false), zh[i].name);
    assert.equal(localizeLevelName(en[i].name, true), en[i].name);
  }
});

test("custom names are preserved", () => {
  for (const en of [true, false]) {
    for (const name of ["自由玩家", "My Cloud Collector", "", "订阅萌新 Plus"]) {
      assert.equal(localizeLevelName(name, en), name);
    }
  }
});
