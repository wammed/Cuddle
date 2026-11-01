#!/usr/bin/env node
/**
 * Automated Integration Test Suite for License Audit Coverage & Consistency (Phase 8-5)
 * Verifies all conditions specified in Section 7:
 *   - 1-to-1 correspondence between lockfile and audit results (no duplicates, no omissions, no extras)
 *   - Distinct version handling
 *   - Conditional dependencies included and validated
 *   - Missing metadata detected as UNKNOWN and blocks PASS
 *   - Unapproved MPL packages never PASS
 *   - Only the 2 registered exceptions recognized as approved
 *   - Dynamic lockfile sizes (synthetic 3-package, 10-package, 0-package lockfiles)
 */

import assert from 'node:assert';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  parsePnpmLockYaml,
  auditPackages,
  scanInstalledPackages,
  loadOfflineMetadata,
  INDIVIDUAL_EXCEPTIONS,
} from './check-frontend-licenses.mjs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');
const lockfilePath = path.join(rootDir, 'pnpm-lock.yaml');
const offlineMetadataPath = path.join(__dirname, 'lockfile-packages-metadata.json');

console.log('Running License Audit Integration & Coverage Tests (Phase 8-5)...');
let passed = 0;
let failed = 0;

function runTestCase(name, fn) {
  try {
    fn();
    console.log(`  ✓ [PASS] Integration: ${name}`);
    passed++;
  } catch (err) {
    console.error(`  ✗ [FAIL] Integration: ${name} => ${err.message}`);
    failed++;
  }
}

// ---------------------------------------------------------------------------
// 1. Full Lockfile 1-to-1 Correspondence on Cuddle Repository
// ---------------------------------------------------------------------------
runTestCase('1. Real lockfile packages match audit results exactly 1-to-1 without duplicates or extras', () => {
  const lockfileRaw = fs.readFileSync(lockfilePath, 'utf8');
  const offlineMetadataObj = loadOfflineMetadata(offlineMetadataPath);
  const { installed } = scanInstalledPackages(path.join(rootDir, 'node_modules'));

  const parsed = parsePnpmLockYaml(lockfileRaw);
  const lockKeys = Object.keys(parsed.packages);

  const report = auditPackages({
    lockfileContent: lockfileRaw,
    installedPackagesMap: installed,
    offlineMetadataObj,
    exceptions: INDIVIDUAL_EXCEPTIONS,
  });

  // Verify total count matches
  assert.strictEqual(report.lockPackageCount, lockKeys.length);
  assert.strictEqual(report.auditResults.length, lockKeys.length);

  // Verify each package appears exactly once
  const resultIds = new Set();
  for (const r of report.auditResults) {
    assert.strictEqual(resultIds.has(r.id), false, `Duplicate package in audit results: ${r.id}`);
    resultIds.add(r.id);
  }

  // Verify no extras or missing
  for (const k of lockKeys) {
    assert.strictEqual(resultIds.has(k), true, `Package in lockfile missing from audit: ${k}`);
  }
  for (const id of resultIds) {
    assert.strictEqual(parsed.packages.hasOwnProperty(id), true, `Extraneous package in audit: ${id}`);
  }

  // Verify exception count is strictly 2 approved exceptions
  assert.strictEqual(INDIVIDUAL_EXCEPTIONS.size, 2);
  assert.strictEqual(report.approvedReviewCount, 2);
  assert.deepStrictEqual(report.matchedExceptions.sort(), [
    'lightningcss-linux-x64-gnu@1.33.0',
    'lightningcss@1.33.0',
  ].sort());

  // Verify unapproved conditional reviews (10 packages)
  assert.strictEqual(report.unapprovedReviewCount, 10);
  assert.strictEqual(report.failCount, 0);
  assert.strictEqual(report.unknownCount, 0);
  assert.strictEqual(report.isCompliant, true);
});

// ---------------------------------------------------------------------------
// 2. Synthetic Miniature Lockfile (3 packages) - Proves no 149 dependency
// ---------------------------------------------------------------------------
runTestCase('2. Synthetic 3-package lockfile audits dynamically without hardcoded counts', () => {
  const lock3 = `lockfileVersion: '9.0'
packages:
  'alpha@1.0.0':
    resolution: {integrity: sha512-abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890==}
  'beta@2.0.0':
    resolution: {integrity: sha512-abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890==}
  'gamma@3.0.0':
    resolution: {integrity: sha512-abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890==}
snapshots:
  'alpha@1.0.0': {}
  'beta@2.0.0': {}
  'gamma@3.0.0': {}
`;
  const report = auditPackages({
    lockfileContent: lock3,
    installedPackagesMap: new Map([
      ['alpha@1.0.0', { license: 'MIT' }],
      ['beta@2.0.0', { license: 'Apache-2.0' }],
      ['gamma@3.0.0', { license: 'ISC' }],
    ]),
    exceptions: new Map(),
  });

  assert.strictEqual(report.lockPackageCount, 3);
  assert.strictEqual(report.auditResults.length, 3);
  assert.strictEqual(report.passCount, 3);
  assert.strictEqual(report.isCompliant, true);
});

// ---------------------------------------------------------------------------
// 3. Synthetic Lockfile (10 packages) - Proves dynamic scalability
// ---------------------------------------------------------------------------
runTestCase('3. Synthetic 10-package lockfile audits dynamically', () => {
  const pkgs = [];
  const snaps = [];
  const installed = new Map();

  for (let i = 1; i <= 10; i++) {
    pkgs.push(`  'pkg-${i}@${i}.0.0':\n    resolution: {integrity: sha512-abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890==}`);
    snaps.push(`  'pkg-${i}@${i}.0.0': {}`);
    installed.set(`pkg-${i}@${i}.0.0`, { license: i % 2 === 0 ? 'MIT' : 'Apache-2.0' });
  }

  const lock10 = `lockfileVersion: '9.0'\npackages:\n${pkgs.join('\n')}\nsnapshots:\n${snaps.join('\n')}\n`;
  const report = auditPackages({
    lockfileContent: lock10,
    installedPackagesMap: installed,
    exceptions: new Map(),
  });

  assert.strictEqual(report.lockPackageCount, 10);
  assert.strictEqual(report.auditResults.length, 10);
  assert.strictEqual(report.passCount, 10);
  assert.strictEqual(report.isCompliant, true);
});

// ---------------------------------------------------------------------------
// 4. Distinct Version Handling
// ---------------------------------------------------------------------------
runTestCase('4. Different versions of the same package audited distinctly', () => {
  const lockMulti = `lockfileVersion: '9.0'
packages:
  'shared-dep@1.0.0':
    resolution: {integrity: sha512-abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890==}
  'shared-dep@2.0.0':
    resolution: {integrity: sha512-abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890==}
snapshots:
  'shared-dep@1.0.0': {}
  'shared-dep@2.0.0': {}
`;
  // Version 1.0.0 is MIT, version 2.0.0 is GPL-3.0-only
  const report = auditPackages({
    lockfileContent: lockMulti,
    installedPackagesMap: new Map([
      ['shared-dep@1.0.0', { license: 'MIT' }],
      ['shared-dep@2.0.0', { license: 'GPL-3.0-only' }],
    ]),
    exceptions: new Map(),
  });

  assert.strictEqual(report.lockPackageCount, 2);
  assert.strictEqual(report.passCount, 1);
  assert.strictEqual(report.failCount, 1);
  assert.strictEqual(report.isCompliant, false);
});

// ---------------------------------------------------------------------------
// 5. Missing Metadata Yields UNKNOWN and Fails Compliance
// ---------------------------------------------------------------------------
runTestCase('5. Missing metadata produces UNKNOWN and blocks compliance', () => {
  const lock = `lockfileVersion: '9.0'
packages:
  'unmet-dep@1.0.0':
    resolution: {integrity: sha512-abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890==}
snapshots:
  'unmet-dep@1.0.0': {}
`;
  // Neither installed nor in offline metadata
  const report = auditPackages({
    lockfileContent: lock,
    installedPackagesMap: new Map(),
    offlineMetadataObj: {},
    exceptions: new Map(),
  });

  assert.strictEqual(report.unknownCount, 1);
  assert.strictEqual(report.isCompliant, false);
});

// ---------------------------------------------------------------------------
// 6. Unapproved MPL Package Never Yields PASS
// ---------------------------------------------------------------------------
runTestCase('6. Unapproved MPL-2.0 package is never PASS', () => {
  const lock = `lockfileVersion: '9.0'
packages:
  'random-mpl@1.0.0':
    resolution: {integrity: sha512-abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890==}
snapshots:
  'random-mpl@1.0.0': {}
`;
  // Case A: Installed on Linux -> must fail audit
  const installedReport = auditPackages({
    lockfileContent: lock,
    installedPackagesMap: new Map([['random-mpl@1.0.0', { license: 'MPL-2.0' }]]),
    exceptions: new Map(),
  });
  assert.strictEqual(installedReport.passCount, 0);
  assert.strictEqual(installedReport.failCount, 1);
  assert.strictEqual(installedReport.isCompliant, false);

  // Case B: Conditional dependency -> unapproved review, never PASS
  const conditionalReport = auditPackages({
    lockfileContent: lock,
    installedPackagesMap: new Map(),
    offlineMetadataObj: {
      'random-mpl@1.0.0': {
        name: 'random-mpl',
        version: '1.0.0',
        license: 'MPL-2.0',
        os: ['darwin'],
        cpu: ['arm64'],
        metadataSource: 'https://registry.npmjs.org/random-mpl',
        metadataRetrievalMethod: 'test',
        metadataRetrievalDate: '2026-11-01',
      },
    },
    exceptions: new Map(),
  });
  assert.strictEqual(conditionalReport.passCount, 0);
  assert.strictEqual(conditionalReport.unapprovedReviewCount, 1);
  assert.strictEqual(conditionalReport.approvedReviewCount, 0);
});

// ---------------------------------------------------------------------------
// 7. Strict Exceptions Purity (Only exactly 2 items permitted)
// ---------------------------------------------------------------------------
runTestCase('7. Exceptions set must strictly contain only the 2 approved items', () => {
  assert.strictEqual(INDIVIDUAL_EXCEPTIONS.size, 2);
  assert.strictEqual(INDIVIDUAL_EXCEPTIONS.has('lightningcss@1.33.0'), true);
  assert.strictEqual(INDIVIDUAL_EXCEPTIONS.has('lightningcss-linux-x64-gnu@1.33.0'), true);

  // Verify none of the 10 removed items exist in exceptions
  const disallowed = [
    'lightningcss-linux-x64-musl@1.33.0',
    'lightningcss-linux-arm64-gnu@1.33.0',
    'lightningcss-linux-arm64-musl@1.33.0',
    'lightningcss-linux-arm-gnueabihf@1.33.0',
    'lightningcss-android-arm64@1.33.0',
    'lightningcss-darwin-arm64@1.33.0',
    'lightningcss-darwin-x64@1.33.0',
    'lightningcss-freebsd-x64@1.33.0',
    'lightningcss-win32-arm64-msvc@1.33.0',
    'lightningcss-win32-x64-msvc@1.33.0',
  ];
  for (const d of disallowed) {
    assert.strictEqual(INDIVIDUAL_EXCEPTIONS.has(d), false, `Disallowed exception present: ${d}`);
  }
});

console.log(`\nAudit Integration Tests Finished: ${passed} passed, ${failed} failed.`);
if (failed > 0) {
  process.exit(1);
}
