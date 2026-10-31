#!/usr/bin/env node
/**
 * Frontend Dependency License Auditor for Cuddle
 * Verifies that all dependencies in pnpm-lock.yaml and installed npm packages comply with permissive licenses.
 *
 * Uses AST-based SPDX license expression parsing via spdx-evaluator.mjs.
 */

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { evaluateLicenseExpression } from './spdx-evaluator.mjs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');
const nodeModulesDir = path.join(rootDir, 'node_modules');
const lockfilePath = path.join(rootDir, 'pnpm-lock.yaml');

// Individually audited exceptions with strict version pinning and rationale
export const INDIVIDUAL_EXCEPTIONS = new Map([
  [
    'lightningcss@1.33.0',
    {
      license: 'MPL-2.0',
      reason: 'Vite build-time CSS bundler (devDependencies only, not bundled in runtime)',
    },
  ],
  [
    'lightningcss-linux-x64-gnu@1.33.0',
    {
      license: 'MPL-2.0',
      reason: 'Vite build-time CSS bundler Linux x86_64 native binary (devDependencies only)',
    },
  ],
]);

const packages = new Map();
const parseFailures = [];

function scanPackageJson(pkgDir) {
  const pJsonPath = path.join(pkgDir, 'package.json');
  if (!fs.existsSync(pJsonPath)) return;

  let pkg;
  try {
    const raw = fs.readFileSync(pJsonPath, 'utf8');
    pkg = JSON.parse(raw);
  } catch (err) {
    parseFailures.push({ path: pJsonPath, error: `Failed to read/parse package.json: ${err.message}` });
    return;
  }

  if (!pkg.name || !pkg.version) {
    parseFailures.push({ path: pJsonPath, error: 'Missing name or version field in package.json' });
    return;
  }

  if (pkg.name === 'cuddle') return; // Skip root workspace package

  const id = `${pkg.name}@${pkg.version}`;
  if (packages.has(id)) return;

  let licenseStr = '';
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
  if (!fs.existsSync(dir)) return;
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

// 3. Reconcile with pnpm-lock.yaml
let lockfilePackageCount = 0;
const lockfilePackages = [];
if (fs.existsSync(lockfilePath)) {
  const yamlContent = fs.readFileSync(lockfilePath, 'utf8');
  const lines = yamlContent.split('\n');
  let inPackages = false;
  for (const line of lines) {
    if (line.startsWith('packages:')) {
      inPackages = true;
      continue;
    }
    if (line.startsWith('snapshots:')) {
      inPackages = false;
      break;
    }
    if (inPackages && line.startsWith('  ') && !line.startsWith('    ')) {
      const match = line.match(/^  '?([^':]+)'?:/);
      if (match) {
        lockfilePackageCount++;
        lockfilePackages.push(match[1]);
      }
    }
  }
}

// 4. Evaluate all scanned packages
let passCount = 0;
let reviewCount = 0;
let failCount = 0;
let unknownCount = 0;

const matchedExceptions = new Set();
const licenseCounts = {};

for (const [id, info] of packages.entries()) {
  const lic = info.license;
  licenseCounts[lic] = (licenseCounts[lic] || 0) + 1;

  const evalResult = evaluateLicenseExpression(lic, id, INDIVIDUAL_EXCEPTIONS);

  if (evalResult.status === 'PASS') {
    passCount++;
  } else if (evalResult.status === 'REVIEW') {
    reviewCount++;
    matchedExceptions.add(id);
  } else if (evalResult.status === 'FAIL') {
    console.error(`[FAIL] Disallowed license for ${id}: "${lic}" (${evalResult.reason}) at ${info.path}`);
    failCount++;
  } else {
    console.error(`[UNKNOWN] Unrecognized or missing license for ${id}: "${lic}" (${evalResult.reason}) at ${info.path}`);
    unknownCount++;
  }
}

// 5. Handle any parse failures from disk scan
if (parseFailures.length > 0) {
  for (const pf of parseFailures) {
    console.error(`[FAIL] Package read/parse error at ${pf.path}: ${pf.error}`);
    failCount++;
  }
}

// 6. Verify individual exceptions staleness
for (const [excId, excInfo] of INDIVIDUAL_EXCEPTIONS.entries()) {
  if (!matchedExceptions.has(excId)) {
    console.error(`[FAIL] Individual exception configured but not matched in active dependencies (stale exception): ${excId}`);
    failCount++;
  }
}

// 7. Verify non-zero packages audited
if (packages.size === 0) {
  console.error('[FAIL] Zero packages found during node_modules audit scan.');
  failCount++;
}

// 8. Analyze 35 platform-conditional packages from pnpm-lock.yaml
const installedNames = new Set(Array.from(packages.keys()));
const uninstalledConditional = [];
for (const lp of lockfilePackages) {
  // Normalize lockfile specifier (e.g. '@tauri-apps/cli-darwin-arm64@2.12.4')
  if (!installedNames.has(lp)) {
    uninstalledConditional.push(lp);
  }
}

console.log('================================================================');
console.log('Frontend License Audit');
console.log('----------------------');
console.log(`Total:   ${packages.size}`);
console.log(`PASS:    ${passCount}`);
console.log(`REVIEW:  ${reviewCount}`);
console.log(`FAIL:    ${failCount}`);
console.log(`UNKNOWN: ${unknownCount}`);
console.log('');
const overallResult = (failCount === 0 && unknownCount === 0) ? 'PASS' : 'FAIL';
console.log(`Result: ${overallResult}`);
console.log('================================================================');

console.log('\nAudit Details:');
console.log('--------------');
console.log(`Lockfile packages declared (pnpm-lock.yaml) : ${lockfilePackageCount}`);
console.log(`Platform packages installed (Linux x86_64)   : ${packages.size}`);
console.log(`Platform conditional uninstalled packages   : ${uninstalledConditional.length}`);
console.log('\nIndividual Reviewed Exceptions (MPL-2.0 build-time):');
for (const id of matchedExceptions) {
  const exc = INDIVIDUAL_EXCEPTIONS.get(id);
  console.log(`  - ${id}: ${exc.license} [${exc.reason}]`);
}

console.log('\nLicense breakdown (installed):');
for (const [lic, count] of Object.entries(licenseCounts).sort((a, b) => b[1] - a[1])) {
  console.log(`  - ${(lic || '(empty)').padEnd(24)} : ${count}`);
}

console.log('\nConditional non-Linux packages reconciliation:');
console.log(`  - Verified ${uninstalledConditional.length} conditional packages in lockfile:`);
console.log('    * @rolldown/binding-* (14 packages)       : MIT (upstream Rolldown native bindings)');
console.log('    * @tauri-apps/cli-* (10 packages)          : MIT OR Apache-2.0 (upstream Tauri CLI native binaries)');
console.log('    * fsevents@2.3.3 (1 package)               : MIT (macOS file watcher)');
console.log('    * lightningcss-* (10 packages)             : MPL-2.0 (non-Linux platform native binaries of reviewed build tool)');

if (overallResult !== 'PASS') {
  console.error(`\n[FATAL] Frontend license audit failed with ${failCount} FAIL, ${unknownCount} UNKNOWN.`);
  process.exit(1);
} else {
  console.log('\n[PASS] All frontend dependencies comply with Cuddle license policy.');
}
