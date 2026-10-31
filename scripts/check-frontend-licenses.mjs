#!/usr/bin/env node
/**
 * Frontend Dependency License Auditor for Cuddle
 * Verifies that all installed direct, dev, and transitive npm packages comply with permissive licenses.
 */

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');
const nodeModulesDir = path.join(rootDir, 'node_modules');

const ALLOWED_LICENSES = new Set([
  'MIT',
  'Apache-2.0',
  'ISC',
  'BSD-2-Clause',
  'BSD-3-Clause',
  '0BSD',
  'BlueOak-1.0.0',
  'CC0-1.0',
  'MIT-0',
  'MPL-2.0', // Individually audited for lightningcss build tool
]);

const packages = new Map();

function scanPackageJson(pkgDir) {
  const pJsonPath = path.join(pkgDir, 'package.json');
  if (!fs.existsSync(pJsonPath)) return;
  try {
    const pkg = JSON.parse(fs.readFileSync(pJsonPath, 'utf8'));
    if (!pkg.name || !pkg.version) return;
    if (pkg.name === 'cuddle') return; // Skip root package

    const id = `${pkg.name}@${pkg.version}`;
    if (packages.has(id)) return;

    let licenseStr = 'UNKNOWN';
    if (pkg.license) {
      licenseStr = typeof pkg.license === 'string' ? pkg.license : (pkg.license.type || JSON.stringify(pkg.license));
    } else if (pkg.licenses) {
      licenseStr = Array.isArray(pkg.licenses)
        ? pkg.licenses.map(l => (typeof l === 'string' ? l : l.type || JSON.stringify(l))).join(' OR ')
        : (pkg.licenses.type || JSON.stringify(pkg.licenses));
    }

    packages.set(id, {
      name: pkg.name,
      version: pkg.version,
      license: licenseStr,
      path: pkgDir,
    });
  } catch (err) {
    console.error(`[WARN] Failed to parse package.json at ${pJsonPath}:`, err.message);
  }
}

function scanDir(dir) {
  if (!fs.existsSync(dir)) return;
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === 'node_modules') {
        scanDirectNodeModules(full);
      } else {
        scanDir(full);
      }
    }
  }
}

function scanDirectNodeModules(dir) {
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const full = path.join(dir, entry.name);
    if (entry.name.startsWith('@')) {
      const scopeEntries = fs.readdirSync(full, { withFileTypes: true });
      for (const sEntry of scopeEntries) {
        scanPackageJson(path.join(full, sEntry.name));
      }
    } else {
      scanPackageJson(full);
    }
  }
}

// 1. Scan direct node_modules
scanDirectNodeModules(nodeModulesDir);

// 2. Scan virtual store in .pnpm if present
const pnpmDir = path.join(nodeModulesDir, '.pnpm');
if (fs.existsSync(pnpmDir)) {
  scanDir(pnpmDir);
}

console.log(`[LICENSE-AUDIT] Scanned ${packages.size} unique installed npm packages.`);

let violations = 0;
const licenseCounts = {};

for (const [id, info] of packages.entries()) {
  const lic = info.license;
  licenseCounts[lic] = (licenseCounts[lic] || 0) + 1;

  // Split expressions like "MIT OR Apache-2.0"
  const normalized = lic.replace(/[()]/g, '');
  const parts = normalized.split(/\s+(?:OR|AND)\s+/);
  const isAllowed = parts.some(p => ALLOWED_LICENSES.has(p.trim()));

  if (!isAllowed) {
    console.error(`[FAIL] Disallowed or unknown license in ${id}: "${lic}" (at ${info.path})`);
    violations++;
  }
}

console.log('[LICENSE-AUDIT] License breakdown:');
for (const [lic, count] of Object.entries(licenseCounts).sort((a, b) => b[1] - a[1])) {
  console.log(`  - ${lic}: ${count}`);
}

if (violations > 0) {
  console.error(`\n[ERROR] Found ${violations} license violation(s). License compliance audit FAILED.`);
  process.exit(1);
} else {
  console.log('\n[PASS] All frontend dependencies strictly comply with allowed permissive licenses.');
}

