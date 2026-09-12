// DSH Dock — carry the browser-side client bundles into dsh's module-proxy
// fallback tree (ADR-0019). Shipped verbatim next to `dsh-boot.mjs`.
//
// WHY THIS EXISTS
// ---------------
// Windows normal accounts cannot create the ~500 symlinks dsh's module fallback
// wants, so the engine bootstrap sets `process.pkg` and dsh materializes its
// link-free "module proxy" tree instead: one real directory per package,
// holding `entry-N.js` ESM re-exports plus a manifest of the form
//
//   { "name", "version", "private", "type", "exports": { ... "./entry-N.js" },
//     "dsh": { "moduleFallback": { "targets": { ... } } } }
//
// That manifest keeps ONLY what Node resolution needs. dsh's *client* module
// system reads a second, unrelated field from the same manifest — the package's
// own `dsh.client` declaration plus its `exports["./client"]` browser bundle —
// and composes the workbench's `__DSH_BOOT__` entry graph from every loader
// entry that declares one. In a packaged dsh build those rows resolve inside
// the executable's own snapshots, so the proxy tree is never consulted; under a
// plain `node` install they resolve *through* the proxy tree, so every
// declaration disappears and the workbench opens with
//
//   Failed to load plugins
//   client-modules: HTML did not preload @deepseek-ai/dsh-client-modules/client.js
//
// This module restores exactly that missing half: for every dsh-managed proxy
// directory, it re-reads the real package the proxy points at, copies that
// package's browser bundle into the proxy directory, and rewrites the proxy
// manifest so the client scan sees the same declarations it would see in
// symlink mode.
//
// CONTRACT KEPT WITH dsh (do not break these three):
//   1. `dsh.moduleFallback.targets` is preserved byte-for-byte: dsh's
//      `moduleFallbackEntryCurrent` / `ensureModuleProxy` idempotence checks
//      compare it, and a change would make dsh delete and rebuild the tree.
//   2. `version` is preserved: same reason.
//   3. Every `entry-N.js` keeps re-exporting the real module by its `file://`
//      URL, so Node's single-instance resolution is unchanged.
// Everything this module writes is additive and idempotent: a second run over
// an already-augmented tree writes nothing at all.
//
// Diagnostics: returns counters instead of throwing, so the caller can log a
// loud drift warning when dsh changes shape (see engines.rs bootstrap).

import { existsSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

/** Browser bundle copy inside a proxy directory. Namespaced so it can never
 * collide with dsh's own `entry-N.js` files. */
export const CLIENT_BUNDLE_NAME = "dsh-client-bundle.js";

/** Read a JSON file, or `undefined` when it is missing/unreadable/malformed. */
function readJson(path) {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch {
    return undefined;
  }
}

/**
 * Walk up from one `file://` target of a proxy manifest to the manifest of the
 * package itself. The proxy points at the real installation tree, so this is
 * how we recover everything the proxy manifest dropped — without reimplementing
 * dsh's dependency-closure walk.
 * @param fileUrl - one `targets` value (`file://…` URL of a real module file).
 * @param packageName - the package name the manifest must carry.
 * @returns the real package directory and its parsed manifest, or `undefined`.
 */
function locateRealPackage(fileUrl, packageName) {
  if (typeof fileUrl !== "string") return undefined;
  let dir;
  try {
    dir = dirname(fileURLToPath(fileUrl));
  } catch {
    return undefined;
  }
  for (;;) {
    const candidate = join(dir, "package.json");
    if (existsSync(candidate)) {
      const manifest = readJson(candidate);
      if (manifest?.name === packageName) return { dir, manifest };
    }
    const parent = dirname(dir);
    if (parent === dir) return undefined;
    dir = parent;
  }
}

/**
 * Resolve `exports["./client"]`, accepting the same string / one-level
 * conditional forms dsh's own client scan accepts.
 * @param exportsField - the real package's `exports` value.
 * @returns the relative bundle path, or `undefined`.
 */
function clientExportOf(exportsField) {
  if (typeof exportsField !== "object" || exportsField === null) return undefined;
  const client = exportsField["./client"];
  if (typeof client === "string") return client;
  if (typeof client === "object" && client !== null && typeof client.default === "string") {
    return client.default;
  }
  return undefined;
}

/** List every candidate package directory under a node_modules root. */
function packageDirs(modulesDir) {
  const out = [];
  let entries;
  try {
    entries = readdirSync(modulesDir, { withFileTypes: true });
  } catch {
    return out;
  }
  for (const entry of entries) {
    if (!entry.isDirectory() || entry.name === ".bin") continue;
    const path = join(modulesDir, entry.name);
    if (entry.name.startsWith("@")) {
      let scoped = [];
      try {
        scoped = readdirSync(path, { withFileTypes: true });
      } catch {
        continue;
      }
      for (const child of scoped) if (child.isDirectory()) out.push(join(path, child.name));
    } else {
      out.push(path);
    }
  }
  return out;
}

/** Copy `source` to `destination` only when the bytes differ (idempotent). */
function copyIfChanged(source, destination) {
  const sourceBytes = readFileSync(source);
  if (existsSync(destination)) {
    try {
      if (readFileSync(destination).equals(sourceBytes)) return false;
    } catch {
      // Unreadable destination: fall through and rewrite it.
    }
  }
  writeFileSync(destination, sourceBytes);
  return true;
}

/**
 * Augment every dsh-managed module proxy under `modulesDir` with the client
 * metadata dsh's proxy manifest omits.
 * @param modulesDir - the fallback root (`$DSH_HOME/profiles/node_modules`).
 * @returns counters for logging; never throws for a single package.
 */
export function augmentClientProxies(modulesDir) {
  const stats = {
    modulesDir,
    scanned: 0,
    proxies: 0,
    client: 0,
    rewritten: 0,
    copied: 0,
    missingSource: [],
    failures: [],
  };
  for (const dir of packageDirs(modulesDir)) {
    stats.scanned++;
    try {
      const manifestPath = join(dir, "package.json");
      const manifest = readJson(manifestPath);
      if (manifest === undefined) continue;
      // Only dsh's own generated proxies are ours to touch: a symlinked or real
      // package directory has no `moduleFallback.targets` shortlist.
      const targets = manifest?.dsh?.moduleFallback?.targets;
      if (typeof targets !== "object" || targets === null) continue;
      stats.proxies++;

      const targetValues = Object.values(targets).filter((value) => typeof value === "string");
      // Prefer `"."` (the package's own entry) before any subpath target.
      const candidates = typeof targets["."] === "string" ? [targets["."], ...targetValues] : targetValues;
      let real;
      for (const value of candidates) {
        real = locateRealPackage(value, manifest.name);
        if (real !== undefined) break;
      }
      if (real === undefined) {
        stats.missingSource.push(manifest.name);
        continue;
      }
      const declaration = real.manifest?.dsh?.client;
      if (declaration === undefined || typeof declaration !== "object" || declaration === null) continue;
      // Only the web platform reaches the browser module table; anything else is
      // dropped by dsh's own scan and must stay untouched here.
      if (declaration.platform !== "web") continue;
      const clientRel = clientExportOf(real.manifest?.exports);
      if (typeof clientRel !== "string") continue;
      const source = join(real.dir, clientRel);
      if (!existsSync(source)) {
        stats.missingSource.push(manifest.name);
        continue;
      }
      stats.client++;

      const bundleCopy = join(dir, CLIENT_BUNDLE_NAME);
      if (copyIfChanged(source, bundleCopy)) stats.copied++;
      const sourceMap = `${source}.map`;
      const mapCopy = `${bundleCopy}.map`;
      if (existsSync(sourceMap)) {
        if (copyIfChanged(sourceMap, mapCopy)) stats.copied++;
      } else if (existsSync(mapCopy)) {
        rmSync(mapCopy, { force: true });
      }

      // Additive rewrite: `dsh.moduleFallback` (and everything else) is carried
      // over verbatim; only `exports["./client"]` and `dsh.client` change. Both
      // are exactly what dsh's client scan reads, and neither participates in
      // its proxy-current checks.
      const next = {
        ...manifest,
        exports: { ...manifest.exports, "./client": `./${CLIENT_BUNDLE_NAME}` },
        dsh: { ...manifest.dsh, client: declaration },
      };
      const rendered = `${JSON.stringify(next, undefined, 2)}\n`;
      if (readFileSync(manifestPath, "utf8") !== rendered) {
        writeFileSync(manifestPath, rendered);
        stats.rewritten++;
      }
    } catch (error) {
      stats.failures.push(`${dir}: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
  return stats;
}
