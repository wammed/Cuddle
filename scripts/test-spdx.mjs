#!/usr/bin/env node
/**
 * Automated Unit Test Suite for Cuddle's SPDX Evaluator
 * Verifies all test cases specified in Phase 6-5.
 */

import assert from 'node:assert';
import { evaluateLicenseExpression } from './spdx-evaluator.mjs';

const individualExceptions = new Map([
  [
    'lightningcss@1.33.0',
    { license: 'MPL-2.0', reason: 'Vite build-time CSS bundler (devDependencies)' }
  ],
  [
    'lightningcss-linux-x64-gnu@1.33.0',
    { license: 'MPL-2.0', reason: 'Vite build-time CSS bundler native binary' }
  ],
]);

const testCases = [
  { expr: 'MIT', pkgId: 'test-pkg@1.0.0', expected: 'PASS' },
  { expr: 'Apache-2.0', pkgId: 'test-pkg@1.0.0', expected: 'PASS' },
  { expr: 'MIT OR Apache-2.0', pkgId: 'test-pkg@1.0.0', expected: 'PASS' },
  { expr: 'MIT AND Apache-2.0', pkgId: 'test-pkg@1.0.0', expected: 'PASS' },
  { expr: 'MIT AND GPL-3.0-only', pkgId: 'test-pkg@1.0.0', expected: 'FAIL' },
  { expr: 'MIT OR GPL-3.0-only', pkgId: 'test-pkg@1.0.0', expected: 'PASS' },
  { expr: 'GPL-3.0-only', pkgId: 'test-pkg@1.0.0', expected: 'FAIL' },
  { expr: 'MPL-2.0', pkgId: 'unregistered-pkg@1.0.0', expected: 'FAIL' },
  { expr: 'MPL-2.0', pkgId: 'lightningcss@1.33.0', expected: 'REVIEW' },
  { expr: 'MPL-2.0', pkgId: 'lightningcss-linux-x64-gnu@1.33.0', expected: 'REVIEW' },
  // Stale exception version test:
  { expr: 'MPL-2.0', pkgId: 'lightningcss@1.34.0', expected: 'FAIL' },
  { expr: 'FooBar-1.0', pkgId: 'test-pkg@1.0.0', expected: 'UNKNOWN' },
  { expr: '', pkgId: 'test-pkg@1.0.0', expected: 'UNKNOWN' },
  { expr: '   ', pkgId: 'test-pkg@1.0.0', expected: 'UNKNOWN' },
  { expr: '(MIT OR)', pkgId: 'test-pkg@1.0.0', expected: 'UNKNOWN' },
  { expr: 'MIT AND', pkgId: 'test-pkg@1.0.0', expected: 'UNKNOWN' },
  { expr: '((MIT)', pkgId: 'test-pkg@1.0.0', expected: 'UNKNOWN' },
];

console.log('Running SPDX Unit Tests...');
let passed = 0;
let failed = 0;

for (const tc of testCases) {
  const result = evaluateLicenseExpression(tc.expr, tc.pkgId, individualExceptions);
  if (result.status === tc.expected) {
    console.log(`  ✓ [${result.status.padEnd(7)}] "${tc.expr}" for ${tc.pkgId}`);
    passed++;
  } else {
    console.error(`  ✗ FAIL: "${tc.expr}" for ${tc.pkgId} => Expected ${tc.expected}, got ${result.status} (${result.reason})`);
    failed++;
  }
}

// Package.json parsing tests (Corrupted JSON & Missing license)
function testPackageJsonEvaluation(jsonStr) {
  try {
    const pkg = JSON.parse(jsonStr);
    if (!pkg.license && !pkg.licenses) {
      return 'UNKNOWN';
    }
    const lic = pkg.license || pkg.licenses;
    return evaluateLicenseExpression(lic, `${pkg.name}@${pkg.version}`, individualExceptions).status;
  } catch (e) {
    return 'FAIL';
  }
}

// Corrupted package.json -> FAIL
const corruptedRes = testPackageJsonEvaluation('{"name": "broken", "version": "1.0.0", invalid}');
if (corruptedRes === 'FAIL') {
  console.log('  ✓ [FAIL   ] Corrupted package.json correctly marked as FAIL');
  passed++;
} else {
  console.error(`  ✗ FAIL: Corrupted package.json expected FAIL, got ${corruptedRes}`);
  failed++;
}

// Missing license in package.json -> UNKNOWN
const missingLicRes = testPackageJsonEvaluation('{"name": "no-license", "version": "1.0.0"}');
if (missingLicRes === 'UNKNOWN') {
  console.log('  ✓ [UNKNOWN] Missing license in package.json correctly marked as UNKNOWN');
  passed++;
} else {
  console.error(`  ✗ FAIL: Missing license expected UNKNOWN, got ${missingLicRes}`);
  failed++;
}

console.log(`\nUnit Tests Finished: ${passed} passed, ${failed} failed.`);
if (failed > 0) {
  process.exit(1);
}

