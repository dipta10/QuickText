// Configuration integration test: mocked invokes cannot catch Tauri ACL failures.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const read = (path) => readFileSync(new URL(path, import.meta.url), "utf8");

describe("main window command permissions", () => {
  it("grants every registered app command through the generated Tauri ACL", () => {
    const backend = read("../src-tauri/src/lib.rs");
    const handlers = backend.match(/tauri::generate_handler!\[([\s\S]*?)\]/)?.[1];
    expect(handlers).toBeDefined();
    const commands = handlers.split(",").map((name) => name.trim()).filter(Boolean);
    const manifests = JSON.parse(read("../src-tauri/gen/schemas/acl-manifests.json"));
    const capabilities = JSON.parse(read("../src-tauri/gen/schemas/capabilities.json"));
    const permissions = manifests["__app-acl__"].permissions;
    const allowed = new Set();
    const denied = new Set();
    for (const capability of Object.values(capabilities)) {
      if (!capability.local || !capability.windows.includes("main")) continue;
      for (const identifier of capability.permissions) {
        const permission = permissions[identifier];
        if (!permission) continue; // Plugin permissions do not grant app commands.
        for (const command of permission.commands.allow) allowed.add(command);
        for (const command of permission.commands.deny) denied.add(command);
      }
    }
    expect(commands.filter((command) => !allowed.has(command) || denied.has(command))).toEqual([]);
  });
});
