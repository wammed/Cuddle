#!/usr/bin/env node
/**
 * Frontend Dependency License Auditor for Cuddle (Phase 7)
 * Verifies all 149 packages declared in pnpm-lock.yaml and installed in node_modules.
 *
 * Uses AST-based SPDX license expression parsing via spdx-evaluator.mjs.
 * Validates real package metadata for installed and conditional dependencies.
 */

import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';
import { evaluateLicenseExpression } from './spdx-evaluator.mjs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');
const nodeModulesDir = path.join(rootDir, 'node_modules');
const lockfilePath = path.join(rootDir, 'pnpm-lock.yaml');
const offlineMetadataPath = path.join(__dirname, 'lockfile-packages-metadata.json');

// Individually audited exceptions with strict version pinning and rationale
// Covers the Vite build-time CSS bundler (lightningcss) and its conditional platform native binaries
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
  [
    'lightningcss-android-arm64@1.33.0',
    {
      license: 'MPL-2.0',
      reason: 'Vite build-time CSS bundler conditional Android arm64 native binary',
    },
  ],
  [
    'lightningcss-darwin-arm64@1.33.0',
    {
      license: 'MPL-2.0',
      reason: 'Vite build-time CSS bundler conditional macOS arm64 native binary',
    },
  ],
  [
    'lightningcss-darwin-x64@1.33.0',
    {
      license: 'MPL-2.0',
      reason: 'Vite build-time CSS bundler conditional macOS x64 native binary',
    },
  ],
  [
    'lightningcss-freebsd-x64@1.33.0',
    {
      license: 'MPL-2.0',
      reason: 'Vite build-time CSS bundler conditional FreeBSD x64 native binary',
    },
  ],
  [
    'lightningcss-linux-arm-gnueabihf@1.33.0',
    {
      license: 'MPL-2.0',
      reason: 'Vite build-time CSS bundler conditional Linux armhf native binary',
    },
  ],
  [
    'lightningcss-linux-arm64-gnu@1.33.0',
    {
      license: 'MPL-2.0',
      reason: 'Vite build-time CSS bundler conditional Linux arm64 glibc native binary',
    },
  ],
  [
    'lightningcss-linux-arm64-musl@1.33.0',
    {
      license: 'MPL-2.0',
      reason: 'Vite build-time CSS bundler conditional Linux arm64 musl native binary',
    },
  ],
  [
    'lightningcss-linux-x64-musl@1.33.0',
    {
      license: 'MPL-2.0',
      reason: 'Vite build-time CSS bundler conditional Linux x64 musl native binary',
    },
  ],
  [
    'lightningcss-win32-arm64-msvc@1.33.0',
    {
      license: 'MPL-2.0',
      reason: 'Vite build-time CSS bundler conditional Windows arm64 native binary',
    },
  ],
  [
    'lightningcss-win32-x64-msvc@1.33.0',
    {
      license: 'MPL-2.0',
      reason: 'Vite build-time CSS bundler conditional Windows x64 native binary',
    },
  ],
]);

// ---------------------------------------------------------------------------
// 1. Structured Lockfile (pnpm-lock.yaml) Parser & Validator
// ---------------------------------------------------------------------------
function parsePnpmLockYaml(content) {
  const lines = content.split('\n');
  const root = {
    lockfileVersion: null,
    importers: {},
    packages: {},
    snapshots: {},
  };

  let currentSection = null;
  let currentKey = null;

  function parseValue(val) {
    const trimmed = val.trim();
    if (trimmed.startsWith('{') && trimmed.endsWith('}')) {
      const inner = trimmed.slice(1, -1).trim();
      const obj = {};
      if (inner.length > 0) {
        for (const pair of inner.split(',')) {
          const colonIdx = pair.indexOf(':');
          if (colonIdx !== -1) {
            obj[pair.slice(0, colonIdx).trim()] = pair.slice(colonIdx + 1).trim().replace(/^['"]|['"]$/g, '');
          }
        }
      }
      return obj;
    }
    if (trimmed.startsWith('[') && trimmed.endsWith(']')) {
      const inner = trimmed.slice(1, -1).trim();
      if (!inner) return [];
      return inner.split(',').map(s => s.trim().replace(/^['"]|['"]$/g, ''));
    }
    if (trimmed === 'true') return true;
    if (trimmed === 'false') return false;
    return trimmed.replace(/^['"]|['"]$/g, '');
  }

  for (let i = 0; i < lines.length; i++) {
    const rawLine = lines[i];
    const line = rawLine.replace(/#.*$/, '').trimEnd();
    if (!line.trim()) continue;

    // Top-level property
    if (!line.startsWith(' ')) {
      const colonIdx = line.indexOf(':');
      if (colonIdx !== -1) {
        const sec = line.slice(0, colonIdx).trim();
        const val = line.slice(colonIdx + 1).trim();
        currentSection = sec;
        currentKey = null;
        if (sec === 'lockfileVersion') {
          root.lockfileVersion = parseValue(val);
        } else if (!root[sec]) {
          root[sec] = {};
        }
      }
      continue;
    }

    // Indent 2: Section entries (e.g. package IDs in packages or snapshots)
    if (line.startsWith('  ') && !line.startsWith('    ')) {
      const trimmed = line.slice(2);
      const colonIdx = trimmed.indexOf(':');
      if (colonIdx !== -1 && currentSection && root[currentSection]) {
        currentKey = trimmed.slice(0, colonIdx).trim().replace(/^['"]|['"]$/g, '');
        const val = trimmed.slice(colonIdx + 1).trim();
        root[currentSection][currentKey] = val ? parseValue(val) : {};
      }
      continue;
    }

    // Indent 4+: Properties inside a section entry
    if (line.startsWith('    ')) {
      if (currentSection && currentKey && root[currentSection] && root[currentSection][currentKey]) {
        const trimmed = line.trim();
        const colonIdx = trimmed.indexOf(':');
        if (colonIdx !== -1) {
          const prop = trimmed.slice(0, colonIdx).trim().replace(/^['"]|['"]$/g, '');
          const val = trimmed.slice(colonIdx + 1).trim();
          root[currentSection][currentKey][prop] = val ? parseValue(val) : {};
        }
      }
    }
  }

  return root;
}

// ---------------------------------------------------------------------------
// 2. Package Metadata Fetcher (Installed node_modules + pnpm cache / offline fallback)
// ---------------------------------------------------------------------------
let offlineMetadata = {};
if (fs.existsSync(offlineMetadataPath)) {
  try {
    offlineMetadata = JSON.parse(fs.readFileSync(offlineMetadataPath, 'utf8'));
  } catch (e) {
    console.error(`[WARN] Failed to parse offline metadata at ${offlineMetadataPath}: ${e.message}`);
  }
}

const pnpmCacheBase = path.join(os.homedir(), '.cache/pnpm/v12/metadata-full/https%3A+registry.npmjs.org');

function readFromPnpmMetadataCache(pkgName, version) {
  let filePath;
  if (pkgName.startsWith('@')) {
    const [scope, name] = pkgName.split('/');
    filePath = path.join(pnpmCacheBase, scope, `${name}.jsonl`);
  } else {
    filePath = path.join(pnpmCacheBase, `${pkgName}.jsonl`);
  }
  if (!fs.existsSync(filePath)) return null;

  try {
    const buf = fs.readFileSync(filePath);
    const newlineIdx = buf.indexOf(10);
    if (newlineIdx === -1) return null;
    const header = buf.subarray(0, newlineIdx).toString('utf8');
    const parts = header.split(' ');
    if (parts[0] !== 'pnpm-meta-v1') return null;
    const len1 = parseInt(parts[1], 10);
    const len2 = parseInt(parts[2], 10);
    const startIdx = newlineIdx + 1;
    const indexObj = JSON.parse(buf.subarray(startIdx + len1, startIdx + len1 + len2).toString('utf8'));
    const vEntry = indexObj.versions ? indexObj.versions.find(v => v[0] === version) : null;
    if (!vEntry) return null;
    const pkgStart = startIdx + len1 + len2 + vEntry[1];
    return JSON.parse(buf.subarray(pkgStart, pkgStart + vEntry[2]).toString('utf8'));
  } catch {
    return null;
  }
}

// ---------------------------------------------------------------------------
// 3. Scan Installed Packages in node_modules
// ---------------------------------------------------------------------------
const installedPackages = new Map();
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
  if (installedPackages.has(id)) return;

  let licenseStr = '';
  if (pkg.license) {
    licenseStr = typeof pkg.license === 'string' ? pkg.license : (pkg.license.type || JSON.stringify(pkg.license));
  } else if (pkg.licenses) {
    licenseStr = Array.isArray(pkg.licenses)
      ? pkg.licenses.map(l => (typeof l === 'string' ? l : l.type || JSON.stringify(l))).join(' OR ')
      : (pkg.licenses.type || JSON.stringify(pkg.licenses));
  }

  installedPackages.set(id, {
    name: pkg.name,
    version: pkg.version,
    license: licenseStr,
    path: pkgDir,
    os: pkg.os || [],
    cpu: pkg.cpu || [],
    source: 'installed',
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

scanDirectNodeModules(nodeModulesDir);
const pnpmDir = path.join(nodeModulesDir, '.pnpm');
if (fs.existsSync(pnpmDir)) {
  scanDir(pnpmDir);
}

// ---------------------------------------------------------------------------
// 4. Lockfile Parsing and Verification
// ---------------------------------------------------------------------------
if (!fs.existsSync(lockfilePath)) {
  console.error(`[FATAL] pnpm-lock.yaml not found at ${lockfilePath}`);
  process.exit(1);
}

const lockfileRaw = fs.readFileSync(lockfilePath, 'utf8');
const parsedLock = parsePnpmLockYaml(lockfileRaw);

if (parsedLock.lockfileVersion !== '9.0' && parsedLock.lockfileVersion !== 9 && parsedLock.lockfileVersion !== '9') {
  console.error(`[FATAL] Unsupported lockfileVersion: ${parsedLock.lockfileVersion} (expected 9.0)`);
  process.exit(1);
}

const lockPackages = Object.keys(parsedLock.packages);
const lockSnapshots = Object.keys(parsedLock.snapshots);

// Verify lockfile integrity
let integrityFailures = 0;
for (const [pkgId, pkgData] of Object.entries(parsedLock.packages)) {
  if (!pkgData.resolution || !pkgData.resolution.integrity) {
    console.error(`[FAIL] Lockfile package ${pkgId} is missing resolution integrity hash!`);
    integrityFailures++;
  }
}

if (lockPackages.length !== 149) {
  console.error(`[FAIL] Expected 149 packages in lockfile, found ${lockPackages.length}`);
  integrityFailures++;
}

// ---------------------------------------------------------------------------
// 5. Evaluate All 149 Packages (Installed & Conditional)
// ---------------------------------------------------------------------------
const allAuditResults = [];
const matchedExceptions = new Set();

let totalPass = 0;
let totalReview = 0;
let totalFail = 0;
let totalUnknown = 0;

let installedCount = 0;
let conditionalCount = 0;

const conditionalDetails = [];

for (const pkgId of lockPackages) {
  const lockData = parsedLock.packages[pkgId] || {};
  const lastAt = pkgId.lastIndexOf('@');
  const name = pkgId.slice(0, lastAt);
  const version = pkgId.slice(lastAt + 1);

  let licenseStr = null;
  let isInstalled = false;
  let metadataOrigin = '';
  let targetOs = lockData.os || [];
  let targetCpu = lockData.cpu || [];

  // Check if installed on Linux x86_64
  if (installedPackages.has(pkgId)) {
    const inst = installedPackages.get(pkgId);
    licenseStr = inst.license;
    isInstalled = true;
    metadataOrigin = `node_modules (${inst.path})`;
    if (inst.os && inst.os.length > 0) targetOs = inst.os;
    if (inst.cpu && inst.cpu.length > 0) targetCpu = inst.cpu;
  } else {
    // Conditional dependency: lookup from real pnpm metadata cache or offline fallback
    let meta = readFromPnpmMetadataCache(name, version);
    if (meta) {
      licenseStr = meta.license || (meta.licenses ? JSON.stringify(meta.licenses) : null);
      metadataOrigin = 'pnpm store cache (~/.cache/pnpm)';
      if (meta.os) targetOs = meta.os;
      if (meta.cpu) targetCpu = meta.cpu;
    } else if (offlineMetadata[pkgId]) {
      const off = offlineMetadata[pkgId];
      licenseStr = off.license;
      metadataOrigin = 'offline metadata snapshot (scripts/lockfile-packages-metadata.json)';
      if (off.os) targetOs = off.os;
      if (off.cpu) targetCpu = off.cpu;
    }
  }

  // Determine condition reason if uninstalled
  let conditionalReason = '';
  let missingInfo = 'None';

  if (!isInstalled) {
    conditionalCount++;
    const osStr = targetOs.length > 0 ? targetOs.join(', ') : 'any';
    const cpuStr = targetCpu.length > 0 ? targetCpu.join(', ') : 'any';

    if (targetOs.length > 0 && !targetOs.includes('linux')) {
      conditionalReason = `Non-Linux OS platform (target os: [${osStr}])`;
    } else if (targetCpu.length > 0 && !targetCpu.includes('x64')) {
      conditionalReason = `Non-x86_64 CPU architecture (target cpu: [${cpuStr}])`;
    } else if (targetOs.includes('linux') && !targetCpu.includes('x64')) {
      conditionalReason = `Non-x86_64 Linux architecture (os: [${osStr}], cpu: [${cpuStr}])`;
    } else {
      conditionalReason = `Conditional build platform (target os: [${osStr}], cpu: [${cpuStr}])`;
    }

    if (!licenseStr) {
      missingInfo = 'Metadata could not be found in local store or cache';
    }
  } else {
    installedCount++;
  }

  // License Evaluation via SPDX Evaluator
  let evalResult;
  if (!licenseStr) {
    evalResult = { status: 'UNKNOWN', reason: `Package license metadata missing for ${pkgId}` };
  } else {
    evalResult = evaluateLicenseExpression(licenseStr, pkgId, INDIVIDUAL_EXCEPTIONS);
  }

  if (evalResult.status === 'PASS') {
    totalPass++;
  } else if (evalResult.status === 'REVIEW') {
    totalReview++;
    matchedExceptions.add(pkgId);
  } else if (evalResult.status === 'FAIL') {
    totalFail++;
  } else {
    totalUnknown++;
  }

  const record = {
    id: pkgId,
    name,
    version,
    license: licenseStr || 'MISSING',
    status: evalResult.status,
    reason: evalResult.reason || '',
    isInstalled,
    metadataOrigin,
    targetOs,
    targetCpu,
    conditionalReason,
    missingInfo,
  };

  allAuditResults.push(record);
  if (!isInstalled) {
    conditionalDetails.push(record);
  }
}

// ---------------------------------------------------------------------------
// 6. Check Parse Failures, Stale Exceptions & Audit Consistency
// ---------------------------------------------------------------------------
let exceptionStalenessFailures = 0;
for (const [excId, excInfo] of INDIVIDUAL_EXCEPTIONS.entries()) {
  if (!matchedExceptions.has(excId)) {
    console.error(`[FAIL] Individual exception configured but not matched in active dependencies (stale exception): ${excId}`);
    exceptionStalenessFailures++;
  }
}

if (parseFailures.length > 0) {
  for (const pf of parseFailures) {
    console.error(`[FAIL] Package read/parse error at ${pf.path}: ${pf.error}`);
  }
}

const totalFailures = totalFail + totalUnknown + parseFailures.length + exceptionStalenessFailures + integrityFailures;
const overallResult = totalFailures === 0 ? 'PASS' : 'FAIL';

// ---------------------------------------------------------------------------
// 7. Structured Report Output
// ---------------------------------------------------------------------------
console.log('================================================================');
console.log('Frontend License Audit (Phase 7: Full 149-Package Coverage)');
console.log('----------------------------------------------------------------');
console.log(`Total:       ${lockPackages.length}`);
console.log(`Installed:   ${installedCount}`);
console.log(`Conditional: ${conditionalCount}`);
console.log('');
console.log(`PASS:        ${totalPass}`);
console.log(`REVIEW:      ${totalReview}`);
console.log(`FAIL:        ${totalFail}`);
console.log(`UNKNOWN:     ${totalUnknown}`);
console.log('');
console.log(`Result: ${overallResult}`);
console.log('================================================================');

console.log('\nConditional Dependencies Audit (35 packages):');
console.log('----------------------------------------------------------------');
for (const c of conditionalDetails) {
  console.log(`Package:        ${c.id}`);
  console.log(`Version:        ${c.version}`);
  console.log(`License Field:  ${c.license}`);
  console.log(`SPDX Result:    ${c.status}${c.reason ? ` (${c.reason})` : ''}`);
  console.log(`Condition:      ${c.conditionalReason}`);
  console.log(`Metadata:       ${c.metadataOrigin}`);
  console.log(`Missing Info:   ${c.missingInfo}`);
  console.log('----------------------------------------------------------------');
}

console.log('\nReviewed Individual Exceptions (12 packages):');
for (const excId of matchedExceptions) {
  const exc = INDIVIDUAL_EXCEPTIONS.get(excId);
  console.log(`  - ${excId}: ${exc.license} [${exc.reason}]`);
}

if (overallResult !== 'PASS') {
  console.error(`\n[FATAL] License audit failed with ${totalFailures} error(s).`);
  process.exit(1);
} else {
  console.log('\n[PASS] All 149 frontend dependencies in pnpm-lock.yaml strictly comply with Cuddle license policy.');
}
