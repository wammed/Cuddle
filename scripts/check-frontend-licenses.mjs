#!/usr/bin/env node
/**
 * Frontend Dependency License Auditor for Cuddle (Phase 8)
 *
 * Verifies all dependencies declared in pnpm-lock.yaml dynamically and with full integrity:
 *   - Strictly limits approved MPL-2.0 individual exceptions to 2 items (Phase 8-1).
 *   - Removes all fixed-count dependencies (no hardcoded 149) (Phase 8-2).
 *   - Enforces 1-to-1 correspondence between `packages` and `snapshots` (Phase 8-2).
 *   - Validates resolution integrity for all packages (Phase 8-2).
 *   - Uses robust YAML parser with syntax and duplicate key validation (Phase 8-3).
 *   - Validates offline metadata provenance, source tracking, and non-empty licenses (Phase 8-4).
 *   - Evaluates all packages via AST-based SPDX evaluator with zero network calls (Phase 8-4).
 */

import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { evaluateLicenseExpression } from './spdx-evaluator.mjs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');
const nodeModulesDir = path.join(rootDir, 'node_modules');
const lockfilePath = path.join(rootDir, 'pnpm-lock.yaml');
const offlineMetadataPath = path.join(__dirname, 'lockfile-packages-metadata.json');

// ---------------------------------------------------------------------------
// Phase 8-1: Strictly Limited Approved Individual Exceptions (Exactly 2 items)
// ---------------------------------------------------------------------------
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

// ---------------------------------------------------------------------------
// Phase 8-2 & Phase 8-3: Structured Lockfile Parser & Validator
// ---------------------------------------------------------------------------
export function parsePnpmLockYaml(content) {
  if (typeof content !== 'string' || !content.trim()) {
    throw new Error('Lockfile content is empty or invalid');
  }

  const lines = content.split('\n');
  const seenSections = new Set();
  const root = {
    lockfileVersion: null,
    _seenSections: seenSections,
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

  const seenKeys = new Map();

  for (let i = 0; i < lines.length; i++) {
    const lineNum = i + 1;
    const rawLine = lines[i];
    const line = rawLine.replace(/#.*$/, '').trimEnd();
    if (!line.trim()) continue;

    // Disallow tab characters in indentation
    if (/^\t+/.test(rawLine)) {
      throw new Error(`YAML syntax error: Tab indentation is disallowed at line ${lineNum}`);
    }

    // Top-level property
    if (!line.startsWith(' ')) {
      const colonIdx = line.indexOf(':');
      if (colonIdx === -1) {
        throw new Error(`YAML syntax error: Missing colon in top-level declaration at line ${lineNum}: "${line}"`);
      }
      const sec = line.slice(0, colonIdx).trim();
      const val = line.slice(colonIdx + 1).trim();
      currentSection = sec;
      currentKey = null;
      seenSections.add(sec);
      if (!seenKeys.has(sec)) seenKeys.set(sec, new Set());

      if (sec === 'lockfileVersion') {
        root.lockfileVersion = parseValue(val);
      } else if (!root[sec]) {
        root[sec] = {};
      }
      continue;
    }

    // Indent 2: Section entries (e.g. package IDs in packages or snapshots)
    if (line.startsWith('  ') && !line.startsWith('    ')) {
      const trimmed = line.slice(2);
      const colonIdx = trimmed.indexOf(':');
      if (colonIdx === -1) {
        throw new Error(`YAML syntax error: Missing colon in entry at line ${lineNum}: "${line}"`);
      }
      if (!currentSection || !root[currentSection]) {
        throw new Error(`YAML syntax error: Entry without parent section at line ${lineNum}: "${line}"`);
      }
      currentKey = trimmed.slice(0, colonIdx).trim().replace(/^['"]|['"]$/g, '');
      const val = trimmed.slice(colonIdx + 1).trim();

      const secSet = seenKeys.get(currentSection);
      if (secSet && secSet.has(currentKey)) {
        throw new Error(`Duplicate key detected in section "${currentSection}": "${currentKey}" at line ${lineNum}`);
      }
      if (secSet) secSet.add(currentKey);

      root[currentSection][currentKey] = val ? parseValue(val) : {};
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

/**
 * Normalizes snapshot keys by stripping peer dependencies suffixes.
 * In pnpm v9: foo@1.0.0(bar@2.0.0)(baz@3.0.0) -> foo@1.0.0
 */
export function normalizeSnapshotKey(key) {
  if (typeof key !== 'string') return '';
  const idx = key.indexOf('(');
  return idx !== -1 ? key.slice(0, idx) : key;
}

/**
 * Validates resolution integrity and format.
 */
export function validateResolution(pkgId, resolution) {
  if (!resolution || typeof resolution !== 'object') {
    return { valid: false, reason: 'Missing resolution object' };
  }
  // Standard registry resolution with integrity hash
  if (resolution.integrity) {
    if (!/^(sha512|sha1|sha256)-[A-Za-z0-9+/=]+$/.test(resolution.integrity)) {
      return { valid: false, reason: `Malformed integrity hash format: "${resolution.integrity}"` };
    }
    return { valid: true };
  }
  // Git repository resolution
  if (resolution.commit && resolution.repo) {
    return { valid: true };
  }
  // Local directory resolution
  if (resolution.directory) {
    return { valid: true };
  }
  // Tarball archive resolution
  if (resolution.tarball) {
    return { valid: true };
  }
  return { valid: false, reason: 'Unrecognized resolution schema' };
}

// ---------------------------------------------------------------------------
// Phase 8-4: Offline Metadata & Store Cache Fetcher
// ---------------------------------------------------------------------------
export function loadOfflineMetadata(filePath = offlineMetadataPath) {
  if (!fs.existsSync(filePath)) {
    return {};
  }
  try {
    const raw = fs.readFileSync(filePath, 'utf8');
    return JSON.parse(raw);
  } catch (err) {
    throw new Error(`Failed to read/parse offline metadata at ${filePath}: ${err.message}`);
  }
}

const pnpmCacheBase = path.join(os.homedir(), '.cache/pnpm/v12/metadata-full/https%3A+registry.npmjs.org');

export function readFromPnpmMetadataCache(pkgName, version, cacheDir = pnpmCacheBase) {
  let filePath;
  if (pkgName.startsWith('@')) {
    const [scope, name] = pkgName.split('/');
    filePath = path.join(cacheDir, scope, `${name}.jsonl`);
  } else {
    filePath = path.join(cacheDir, `${pkgName}.jsonl`);
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
// Node_modules Scanner
// ---------------------------------------------------------------------------
export function scanInstalledPackages(baseNodeModulesDir = nodeModulesDir) {
  const installed = new Map();
  const parseFailures = [];

  function scanPkgDir(pkgDir) {
    const pJsonPath = path.join(pkgDir, 'package.json');
    if (!fs.existsSync(pJsonPath)) return;

    let pkg;
    try {
      pkg = JSON.parse(fs.readFileSync(pJsonPath, 'utf8'));
    } catch (err) {
      parseFailures.push({ path: pJsonPath, error: `JSON parse error: ${err.message}` });
      return;
    }

    if (!pkg.name || !pkg.version) {
      parseFailures.push({ path: pJsonPath, error: 'Missing name or version in package.json' });
      return;
    }

    if (pkg.name === 'cuddle') return; // Skip workspace root

    const id = `${pkg.name}@${pkg.version}`;
    if (installed.has(id)) return;

    let licenseStr = '';
    if (pkg.license) {
      licenseStr = typeof pkg.license === 'string' ? pkg.license : (pkg.license.type || JSON.stringify(pkg.license));
    } else if (pkg.licenses) {
      licenseStr = Array.isArray(pkg.licenses)
        ? pkg.licenses.map(l => (typeof l === 'string' ? l : l.type || JSON.stringify(l))).join(' OR ')
        : (pkg.licenses.type || JSON.stringify(pkg.licenses));
    }

    installed.set(id, {
      name: pkg.name,
      version: pkg.version,
      license: licenseStr,
      path: pkgDir,
      os: pkg.os || [],
      cpu: pkg.cpu || [],
    });
  }

  function scanDirect(dir) {
    if (!fs.existsSync(dir)) return;
    for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, ent.name);
      if (ent.name.startsWith('@')) {
        for (const sEnt of fs.readdirSync(full, { withFileTypes: true })) {
          scanPkgDir(path.join(full, sEnt.name));
        }
      } else {
        scanPkgDir(full);
      }
    }
  }

  scanDirect(baseNodeModulesDir);
  const pnpmStore = path.join(baseNodeModulesDir, '.pnpm');
  if (fs.existsSync(pnpmStore)) {
    for (const ent of fs.readdirSync(pnpmStore, { withFileTypes: true })) {
      if (!ent.isDirectory()) continue;
      const nested = path.join(pnpmStore, ent.name, 'node_modules');
      if (fs.existsSync(nested)) {
        scanDirect(nested);
      }
    }
  }

  return { installed, parseFailures };
}

// ---------------------------------------------------------------------------
// Core Audit Engine (Phase 8-1 through Phase 8-5)
// ---------------------------------------------------------------------------
export function auditPackages({
  lockfileContent,
  installedPackagesMap,
  offlineMetadataObj,
  exceptions = INDIVIDUAL_EXCEPTIONS,
}) {
  const parsedLock = parsePnpmLockYaml(lockfileContent);

  // Validate Lockfile Version
  const v = String(parsedLock.lockfileVersion);
  if (v !== '9.0' && v !== '9') {
    throw new Error(`Unsupported lockfileVersion: "${parsedLock.lockfileVersion}" (expected 9.0)`);
  }

  // Validate Sections
  if (!parsedLock._seenSections || !parsedLock._seenSections.has('packages')) {
    throw new Error('Lockfile missing "packages" section');
  }
  const lockPackageKeys = Object.keys(parsedLock.packages || {});
  if (lockPackageKeys.length === 0) {
    throw new Error('Lockfile "packages" section is empty');
  }

  if (!parsedLock._seenSections.has('snapshots')) {
    throw new Error('Lockfile missing "snapshots" section');
  }
  const snapshotKeys = Object.keys(parsedLock.snapshots || {});
  if (snapshotKeys.length === 0) {
    throw new Error('Lockfile "snapshots" section is empty while packages exist');
  }

  // Phase 8-2: Validate Packages <-> Snapshots 1-to-1 Correspondence
  const normalizedSnapshotSet = new Set(snapshotKeys.map(normalizeSnapshotKey));
  const lockPackageSet = new Set(lockPackageKeys);

  const missingInSnapshots = lockPackageKeys.filter(pkgKey => !normalizedSnapshotSet.has(pkgKey));
  const missingInPackages = Array.from(normalizedSnapshotSet).filter(snapKey => !lockPackageSet.has(snapKey));

  const integrityErrors = [];
  if (missingInSnapshots.length > 0) {
    integrityErrors.push(`Packages missing in snapshots: ${missingInSnapshots.join(', ')}`);
  }
  if (missingInPackages.length > 0) {
    integrityErrors.push(`Snapshots missing in packages: ${missingInPackages.join(', ')}`);
  }

  // Phase 8-2: Validate Package Keys and Resolutions
  for (const pkgId of lockPackageKeys) {
    const lastAt = pkgId.lastIndexOf('@');
    if (lastAt <= 0 || lastAt === pkgId.length - 1) {
      integrityErrors.push(`Invalid package key in lockfile: "${pkgId}"`);
      continue;
    }

    const pkgData = parsedLock.packages[pkgId];
    const resCheck = validateResolution(pkgId, pkgData ? pkgData.resolution : null);
    if (!resCheck.valid) {
      integrityErrors.push(`Resolution error for ${pkgId}: ${resCheck.reason}`);
    }
  }

  // Phase 8-4: Validate Offline Metadata Keys
  const metadataErrors = [];
  if (offlineMetadataObj && typeof offlineMetadataObj === 'object') {
    for (const [metaKey, metaVal] of Object.entries(offlineMetadataObj)) {
      if (!lockPackageSet.has(metaKey)) {
        metadataErrors.push(`Extraneous metadata entry not declared in lockfile: "${metaKey}"`);
      }
      const lastAt = metaKey.lastIndexOf('@');
      const expectedName = metaKey.slice(0, lastAt);
      const expectedVersion = metaKey.slice(lastAt + 1);

      if (metaVal.name !== expectedName || metaVal.version !== expectedVersion) {
        metadataErrors.push(`Metadata name/version mismatch for "${metaKey}": got "${metaVal.name}@${metaVal.version}"`);
      }
      if (!metaVal.license || metaVal.license.trim() === '') {
        metadataErrors.push(`Metadata license missing or empty for "${metaKey}"`);
      }
      if (!metaVal.metadataSource) {
        metadataErrors.push(`Metadata source URL missing for "${metaKey}"`);
      }
      if (!metaVal.metadataRetrievalMethod) {
        metadataErrors.push(`Metadata retrieval method missing for "${metaKey}"`);
      }
      if (!metaVal.metadataRetrievalDate) {
        metadataErrors.push(`Metadata retrieval date missing for "${metaKey}"`);
      }
    }
  }

  // Phase 8-1, 8-2, 8-4: Evaluate all packages dynamically
  const auditResults = [];
  const matchedExceptions = new Set();

  let passCount = 0;
  let approvedReviewCount = 0;
  let unapprovedReviewCount = 0;
  let failCount = 0;
  let unknownCount = 0;
  let installedCount = 0;
  let conditionalCount = 0;

  for (const pkgId of lockPackageKeys) {
    const lockData = parsedLock.packages[pkgId] || {};
    const lastAt = pkgId.lastIndexOf('@');
    const name = pkgId.slice(0, lastAt);
    const version = pkgId.slice(lastAt + 1);

    let licenseStr = null;
    let isInstalled = false;
    let metadataOrigin = '';
    let targetOs = lockData.os || [];
    let targetCpu = lockData.cpu || [];

    if (installedPackagesMap && installedPackagesMap.has(pkgId)) {
      const inst = installedPackagesMap.get(pkgId);
      licenseStr = inst.license;
      isInstalled = true;
      metadataOrigin = `node_modules (${inst.path})`;
      if (inst.os && inst.os.length > 0) targetOs = inst.os;
      if (inst.cpu && inst.cpu.length > 0) targetCpu = inst.cpu;
    } else {
      // Lookup from local store cache or verified offline metadata
      let meta = readFromPnpmMetadataCache(name, version);
      if (meta) {
        licenseStr = meta.license || (meta.licenses ? JSON.stringify(meta.licenses) : null);
        metadataOrigin = 'pnpm store cache (~/.cache/pnpm)';
        if (meta.os) targetOs = meta.os;
        if (meta.cpu) targetCpu = meta.cpu;
      } else if (offlineMetadataObj && offlineMetadataObj[pkgId]) {
        const off = offlineMetadataObj[pkgId];
        licenseStr = off.license;
        metadataOrigin = `offline metadata snapshot (${off.metadataSource})`;
        if (off.os) targetOs = off.os;
        if (off.cpu) targetCpu = off.cpu;
      }
    }

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
        missingInfo = 'Metadata could not be found in local store or offline metadata';
      }
    } else {
      installedCount++;
    }

    let evalResult;
    if (!licenseStr) {
      evalResult = { status: 'UNKNOWN', reason: `Package license metadata missing for ${pkgId}` };
    } else {
      evalResult = evaluateLicenseExpression(licenseStr, pkgId, exceptions);
    }

    if (evalResult.status === 'PASS') {
      passCount++;
    } else if (evalResult.status === 'REVIEW') {
      if (evalResult.approved) {
        approvedReviewCount++;
        matchedExceptions.add(pkgId);
      } else {
        unapprovedReviewCount++;
        // If an unapproved copyleft package is INSTALLED on Linux, it is a fatal failure
        if (isInstalled) {
          failCount++;
        }
      }
    } else if (evalResult.status === 'FAIL') {
      failCount++;
    } else {
      unknownCount++;
    }

    auditResults.push({
      id: pkgId,
      name,
      version,
      license: licenseStr || 'MISSING',
      status: evalResult.status,
      approved: evalResult.approved ?? false,
      reason: evalResult.reason || '',
      isInstalled,
      metadataOrigin,
      targetOs,
      targetCpu,
      conditionalReason,
      missingInfo,
    });
  }

  // Phase 8-1: Stale Exception Detection
  const staleExceptions = [];
  for (const [excId] of exceptions.entries()) {
    if (!matchedExceptions.has(excId)) {
      staleExceptions.push(excId);
    }
  }

  const isCompliant =
    failCount === 0 &&
    unknownCount === 0 &&
    integrityErrors.length === 0 &&
    metadataErrors.length === 0 &&
    staleExceptions.length === 0 &&
    auditResults.length === lockPackageKeys.length &&
    lockPackageKeys.length > 0;

  return {
    lockPackageCount: lockPackageKeys.length,
    installedCount,
    conditionalCount,
    passCount,
    approvedReviewCount,
    unapprovedReviewCount,
    totalReviewCount: approvedReviewCount + unapprovedReviewCount,
    failCount,
    unknownCount,
    integrityErrors,
    metadataErrors,
    staleExceptions,
    matchedExceptions: Array.from(matchedExceptions),
    auditResults,
    isCompliant,
  };
}

// ---------------------------------------------------------------------------
// CLI Execution
// ---------------------------------------------------------------------------
export function runCli() {
  if (!fs.existsSync(lockfilePath)) {
    console.error(`[FATAL] pnpm-lock.yaml not found at ${lockfilePath}`);
    process.exit(1);
  }

  let lockfileRaw;
  try {
    lockfileRaw = fs.readFileSync(lockfilePath, 'utf8');
  } catch (err) {
    console.error(`[FATAL] Could not read lockfile: ${err.message}`);
    process.exit(1);
  }

  let offlineMetadataObj;
  try {
    offlineMetadataObj = loadOfflineMetadata(offlineMetadataPath);
  } catch (err) {
    console.error(`[FATAL] ${err.message}`);
    process.exit(1);
  }

  const { installed, parseFailures } = scanInstalledPackages(nodeModulesDir);
  if (parseFailures.length > 0) {
    for (const pf of parseFailures) {
      console.error(`[FAIL] Package read/parse error at ${pf.path}: ${pf.error}`);
    }
  }

  let report;
  try {
    report = auditPackages({
      lockfileContent: lockfileRaw,
      installedPackagesMap: installed,
      offlineMetadataObj,
      exceptions: INDIVIDUAL_EXCEPTIONS,
    });
  } catch (err) {
    console.error(`[FATAL] Lockfile audit failed: ${err.message}`);
    process.exit(1);
  }

  for (const err of report.integrityErrors) {
    console.error(`[FAIL] Lockfile integrity: ${err}`);
  }
  for (const err of report.metadataErrors) {
    console.error(`[FAIL] Metadata verification: ${err}`);
  }
  for (const exc of report.staleExceptions) {
    console.error(`[FAIL] Stale approved exception (not encountered): ${exc}`);
  }

  const totalErrors =
    report.failCount +
    report.unknownCount +
    report.integrityErrors.length +
    report.metadataErrors.length +
    report.staleExceptions.length +
    parseFailures.length;

  const overallResult = report.isCompliant && totalErrors === 0 ? 'PASS' : 'FAIL';

  console.log('================================================================');
  console.log('Frontend License Audit (Phase 8: Dynamic Lockfile & Strict MPL)');
  console.log('----------------------------------------------------------------');
  console.log(`Total:       ${report.lockPackageCount}`);
  console.log(`Installed:   ${report.installedCount}`);
  console.log(`Conditional: ${report.conditionalCount}`);
  console.log('');
  console.log(`PASS:        ${report.passCount}`);
  console.log(`REVIEW:      ${report.totalReviewCount}`);
  console.log(`  - Approved Individual Exceptions : ${report.approvedReviewCount}`);
  console.log(`  - Unapproved Conditional Reviews : ${report.unapprovedReviewCount}`);
  console.log(`FAIL:        ${report.failCount}`);
  console.log(`UNKNOWN:     ${report.unknownCount}`);
  console.log('');
  console.log(`Result: ${overallResult}`);
  console.log('================================================================');

  console.log('\nApproved Individual Exceptions (Phase 8-1: Strictly 2 items):');
  for (const excId of report.matchedExceptions) {
    const exc = INDIVIDUAL_EXCEPTIONS.get(excId);
    console.log(`  - ${excId}: ${exc.license} [${exc.reason}]`);
  }

  console.log('\nConditional Dependencies Audit:');
  console.log('----------------------------------------------------------------');
  const conditionalItems = report.auditResults.filter(r => !r.isInstalled);
  for (const c of conditionalItems) {
    const reviewNote = c.status === 'REVIEW' ? (c.approved ? ' [Approved Exception]' : ' [Unapproved Review]') : '';
    console.log(`Package:        ${c.id}`);
    console.log(`Version:        ${c.version}`);
    console.log(`License Field:  ${c.license}`);
    console.log(`SPDX Result:    ${c.status}${reviewNote}`);
    console.log(`Condition:      ${c.conditionalReason}`);
    console.log(`Metadata:       ${c.metadataOrigin}`);
    console.log(`Missing Info:   ${c.missingInfo}`);
    console.log('----------------------------------------------------------------');
  }

  if (overallResult !== 'PASS') {
    console.error(`\n[FATAL] License audit failed with ${totalErrors} error(s).`);
    process.exit(1);
  } else {
    console.log(`\n[PASS] All ${report.lockPackageCount} frontend dependencies in pnpm-lock.yaml strictly comply with Cuddle license policy.`);
  }
}

// Run CLI when invoked directly
if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  runCli();
}
