#!/usr/bin/env node
/**
 * Automated Unit Test Suite for Lockfile Parser & Integrity Verification (Phase 8-3)
 * Tests all 15 required abnormal cases and normal verification specified in Section 5.2.
 */

import assert from 'node:assert';
import {
  parsePnpmLockYaml,
  normalizeSnapshotKey,
  validateResolution,
  auditPackages,
  INDIVIDUAL_EXCEPTIONS,
} from './check-frontend-licenses.mjs';

const VALID_MINIMAL_LOCKFILE = `lockfileVersion: '9.0'

settings:
  autoInstallPeers: true

importers:
  .:
    dependencies:
      pkg-a:
        specifier: ^1.0.0
        version: 1.0.0

packages:
  'pkg-a@1.0.0':
    resolution: {integrity: sha512-abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890==}

snapshots:
  'pkg-a@1.0.0': {}
`;

console.log('Running Lockfile Parser Abnormal & Edge Case Tests (Phase 8-3)...');
let passed = 0;
let failed = 0;

function runTestCase(name, fn) {
  try {
    fn();
    console.log(`  ✓ [PASS] Case: ${name}`);
    passed++;
  } catch (err) {
    console.error(`  ✗ [FAIL] Case: ${name} => ${err.message}`);
    failed++;
  }
}

// 1. Lockfile does not exist (handled at caller/cli level)
runTestCase('1. Lockfile content is null / empty', () => {
  assert.throws(
    () => parsePnpmLockYaml(''),
    /empty or invalid/i
  );
});

// 2. YAML syntax corrupted (e.g. tab indentation)
runTestCase('2. YAML syntax corrupted (tab characters in indentation)', () => {
  const corrupted = "lockfileVersion: '9.0'\npackages:\n\t'pkg@1.0.0':\n";
  assert.throws(
    () => parsePnpmLockYaml(corrupted),
    /Tab indentation is disallowed/i
  );
});

// 3. lockfileVersion missing
runTestCase('3. lockfileVersion missing', () => {
  const noVersion = `packages:
  'pkg-a@1.0.0':
    resolution: {integrity: sha512-abc==}
snapshots:
  'pkg-a@1.0.0': {}
`;
  assert.throws(
    () => auditPackages({ lockfileContent: noVersion }),
    /Unsupported lockfileVersion/i
  );
});

// 4. Unsupported lockfileVersion (e.g. 5.4 or 6.0)
runTestCase('4. Unsupported lockfileVersion (e.g. 5.4)', () => {
  const badVersion = `lockfileVersion: '5.4'
packages:
  'pkg-a@1.0.0':
    resolution: {integrity: sha512-abc==}
snapshots:
  'pkg-a@1.0.0': {}
`;
  assert.throws(
    () => auditPackages({ lockfileContent: badVersion }),
    /Unsupported lockfileVersion: "5.4"/i
  );
});

// 5. packages section missing
runTestCase('5. packages section missing', () => {
  const noPkgs = `lockfileVersion: '9.0'
snapshots:
  'pkg-a@1.0.0': {}
`;
  assert.throws(
    () => auditPackages({ lockfileContent: noPkgs }),
    /missing "packages" section/i
  );
});

// 6. packages section empty
runTestCase('6. packages section empty', () => {
  const emptyPkgs = `lockfileVersion: '9.0'
packages:
snapshots:
  'pkg-a@1.0.0': {}
`;
  assert.throws(
    () => auditPackages({ lockfileContent: emptyPkgs }),
    /"packages" section is empty/i
  );
});

// 7. snapshots section missing
runTestCase('7. snapshots section missing', () => {
  const noSnapshots = `lockfileVersion: '9.0'
packages:
  'pkg-a@1.0.0':
    resolution: {integrity: sha512-abc==}
`;
  assert.throws(
    () => auditPackages({ lockfileContent: noSnapshots }),
    /missing "snapshots" section/i
  );
});

// 8. snapshots empty while packages present
runTestCase('8. snapshots empty while packages present', () => {
  const emptySnapshots = `lockfileVersion: '9.0'
packages:
  'pkg-a@1.0.0':
    resolution: {integrity: sha512-abc==}
snapshots:
`;
  assert.throws(
    () => auditPackages({ lockfileContent: emptySnapshots }),
    /"snapshots" section is empty while packages exist/i
  );
});

// 9. packages and snapshots correspondence broken
runTestCase('9. packages and snapshots mismatch (orphan package)', () => {
  const mismatch = `lockfileVersion: '9.0'
packages:
  'pkg-a@1.0.0':
    resolution: {integrity: sha512-abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890==}
  'pkg-b@2.0.0':
    resolution: {integrity: sha512-abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890==}
snapshots:
  'pkg-a@1.0.0': {}
`;
  const result = auditPackages({
    lockfileContent: mismatch,
    installedPackagesMap: new Map([
      ['pkg-a@1.0.0', { license: 'MIT' }],
      ['pkg-b@2.0.0', { license: 'MIT' }],
    ]),
  });
  assert.strictEqual(result.isCompliant, false);
  assert.ok(result.integrityErrors.some(e => e.includes('pkg-b@2.0.0')));
});

// 10. Bad indentation or entry without parent section
runTestCase('10. Entry without parent section', () => {
  const badIndent = `  'dangling-entry@1.0.0':\n`;
  assert.throws(
    () => parsePnpmLockYaml(badIndent),
    /Entry without parent section/i
  );
});

// 11. Multiple versions of the same package handled distinctly
runTestCase('11. Multiple versions of same package audited independently', () => {
  const multiVersion = `lockfileVersion: '9.0'
packages:
  'shared-lib@1.0.0':
    resolution: {integrity: sha512-abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890==}
  'shared-lib@2.0.0':
    resolution: {integrity: sha512-abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890==}
snapshots:
  'shared-lib@1.0.0': {}
  'shared-lib@2.0.0': {}
`;
  const result = auditPackages({
    lockfileContent: multiVersion,
    installedPackagesMap: new Map([
      ['shared-lib@1.0.0', { license: 'MIT' }],
      ['shared-lib@2.0.0', { license: 'Apache-2.0' }],
    ]),
    exceptions: new Map(),
  });
  assert.strictEqual(result.isCompliant, true);
  assert.strictEqual(result.lockPackageCount, 2);
  assert.strictEqual(result.passCount, 2);
});

// 12. Invalid package key format (e.g. missing @ or version)
runTestCase('12. Invalid package key in lockfile', () => {
  const invalidKey = `lockfileVersion: '9.0'
packages:
  'invalid-package-no-version':
    resolution: {integrity: sha512-abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890==}
snapshots:
  'invalid-package-no-version': {}
`;
  const result = auditPackages({
    lockfileContent: invalidKey,
    installedPackagesMap: new Map([['invalid-package-no-version', { license: 'MIT' }]]),
  });
  assert.strictEqual(result.isCompliant, false);
  assert.ok(result.integrityErrors.some(e => e.includes('Invalid package key')));
});

// 13. Invalid resolution (missing integrity / corrupted format)
runTestCase('13. Malformed resolution integrity hash', () => {
  const badRes = `lockfileVersion: '9.0'
packages:
  'pkg-a@1.0.0':
    resolution: {integrity: not-a-valid-hash}
snapshots:
  'pkg-a@1.0.0': {}
`;
  const result = auditPackages({
    lockfileContent: badRes,
    installedPackagesMap: new Map([['pkg-a@1.0.0', { license: 'MIT' }]]),
  });
  assert.strictEqual(result.isCompliant, false);
  assert.ok(result.integrityErrors.some(e => e.includes('Resolution error')));
});

// 14. Missing required metadata (name/version mismatch in offline cache)
runTestCase('14. Missing required metadata or license', () => {
  const lock = `lockfileVersion: '9.0'
packages:
  'conditional-pkg@1.0.0':
    resolution: {integrity: sha512-abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890==}
snapshots:
  'conditional-pkg@1.0.0': {}
`;
  // No metadata provided -> UNKNOWN
  const result = auditPackages({
    lockfileContent: lock,
    installedPackagesMap: new Map(),
    offlineMetadataObj: {},
  });
  assert.strictEqual(result.isCompliant, false);
  assert.strictEqual(result.unknownCount, 1);
});

// 15. Duplicate key definition in lockfile
runTestCase('15. Duplicate key definition in section', () => {
  const dupKeys = `lockfileVersion: '9.0'
packages:
  'pkg-a@1.0.0':
    resolution: {integrity: sha512-abc1==}
  'pkg-a@1.0.0':
    resolution: {integrity: sha512-abc2==}
snapshots:
  'pkg-a@1.0.0': {}
`;
  assert.throws(
    () => parsePnpmLockYaml(dupKeys),
    /Duplicate key detected in section "packages": "pkg-a@1\.0\.0"/i
  );
});

// 16. Normal valid lockfile
runTestCase('16. Valid minimal pnpm v9 lockfile passes verification', () => {
  const result = auditPackages({
    lockfileContent: VALID_MINIMAL_LOCKFILE,
    installedPackagesMap: new Map([['pkg-a@1.0.0', { license: 'MIT' }]]),
    exceptions: new Map(),
  });
  assert.strictEqual(result.isCompliant, true);
  assert.strictEqual(result.lockPackageCount, 1);
  assert.strictEqual(result.passCount, 1);
  assert.strictEqual(result.failCount, 0);
  assert.strictEqual(result.unknownCount, 0);
  assert.strictEqual(result.integrityErrors.length, 0);
});

console.log(`\nLockfile Parser Tests Finished: ${passed} passed, ${failed} failed.`);
if (failed > 0) {
  process.exit(1);
}
