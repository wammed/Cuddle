#!/usr/bin/env node
/**
 * Automated Unit Test Suite for Cuddle's SPDX Evaluator (Phase 7)
 * Verifies all formal SPDX 3.0 grammar, identifier validation, and policy compliance cases.
 */

import { evaluateLicenseExpression } from './spdx-evaluator.mjs';

const individualExceptions = new Map([
  [
    'lightningcss@1.33.0',
    { license: 'MPL-2.0', reason: 'Vite build-time CSS bundler (devDependencies)' },
  ],
  [
    'lightningcss-linux-x64-gnu@1.33.0',
    { license: 'MPL-2.0', reason: 'Vite build-time CSS bundler native binary' },
  ],
]);

const testCases = [
  // 1. Basic Permissive Licenses
  { expr: 'MIT', pkgId: 'test-pkg@1.0.0', expected: 'PASS', desc: 'Single permissive license' },
  { expr: 'Apache-2.0', pkgId: 'test-pkg@1.0.0', expected: 'PASS', desc: 'Single permissive license' },
  { expr: 'MIT OR Apache-2.0', pkgId: 'test-pkg@1.0.0', expected: 'PASS', desc: 'Disjunctive permissive choice' },
  { expr: 'MIT AND Apache-2.0', pkgId: 'test-pkg@1.0.0', expected: 'PASS', desc: 'Conjunctive permissive licenses' },

  // 2. Copyleft & Policy Combinations
  { expr: 'MIT AND GPL-3.0-only', pkgId: 'test-pkg@1.0.0', expected: 'FAIL', desc: 'AND with copyleft license' },
  { expr: 'MIT OR GPL-3.0-only', pkgId: 'test-pkg@1.0.0', expected: 'PASS', desc: 'OR with permissive choice allowed' },
  { expr: 'GPL-3.0-only', pkgId: 'test-pkg@1.0.0', expected: 'FAIL', desc: 'Standalone copyleft license' },

  // 3. MPL-2.0 Individual Exception Evaluation
  { expr: 'MPL-2.0', pkgId: 'unregistered-pkg@1.0.0', expected: 'FAIL', desc: 'Unregistered MPL-2.0 package' },
  { expr: 'MPL-2.0', pkgId: 'lightningcss@1.33.0', expected: 'REVIEW', desc: 'Registered individual exception' },
  { expr: 'MPL-2.0', pkgId: 'lightningcss-linux-x64-gnu@1.33.0', expected: 'REVIEW', desc: 'Registered native exception' },
  { expr: 'MPL-2.0', pkgId: 'lightningcss@1.34.0', expected: 'FAIL', desc: 'Version bump without updated review' },

  // 4. Non-SPDX Operators (Requirement 3.1)
  { expr: 'MIT || Apache-2.0', pkgId: 'test-pkg@1.0.0', expected: 'UNKNOWN', desc: 'Rejection of non-SPDX || operator' },
  { expr: 'MIT && Apache-2.0', pkgId: 'test-pkg@1.0.0', expected: 'UNKNOWN', desc: 'Rejection of non-SPDX && operator' },

  // 5. WITH Scope Constraints (Requirement 3.2)
  {
    expr: '(MIT OR Apache-2.0) WITH LLVM-exception',
    pkgId: 'test-pkg@1.0.0',
    expected: 'UNKNOWN',
    desc: 'Rejection of WITH on compound parenthesized expression',
  },
  {
    expr: '(MIT) WITH LLVM-exception',
    pkgId: 'test-pkg@1.0.0',
    expected: 'UNKNOWN',
    desc: 'Rejection of WITH on simple parenthesized expression',
  },

  // 6. WITH Exception Combinations (Requirement 3.3)
  {
    expr: 'Apache-2.0 WITH LLVM-exception',
    pkgId: 'test-pkg@1.0.0',
    expected: 'PASS',
    desc: 'Approved Apache-2.0 WITH LLVM-exception combination',
  },
  {
    expr: 'MIT WITH LLVM-exception',
    pkgId: 'test-pkg@1.0.0',
    expected: 'UNKNOWN',
    desc: 'Unapproved MIT WITH LLVM-exception combination',
  },
  {
    expr: 'Apache-2.0 WITH Unknown-exception',
    pkgId: 'test-pkg@1.0.0',
    expected: 'UNKNOWN',
    desc: 'Unrecognized SPDX exception identifier',
  },

  // 7. Unknown Identifiers in OR Expressions (Requirement 3.4)
  {
    expr: 'MIT OR FooBar-1.0',
    pkgId: 'test-pkg@1.0.0',
    expected: 'UNKNOWN',
    desc: 'Unknown license identifier in OR branch must not be silently passed',
  },
  {
    expr: 'FooBar-1.0 OR MIT',
    pkgId: 'test-pkg@1.0.0',
    expected: 'UNKNOWN',
    desc: 'Unknown license identifier on left of OR',
  },
  {
    expr: 'MIT AND FooBar-1.0',
    pkgId: 'test-pkg@1.0.0',
    expected: 'UNKNOWN',
    desc: 'Unknown license identifier in AND expression',
  },
  {
    expr: 'FooBar-1.0',
    pkgId: 'test-pkg@1.0.0',
    expected: 'UNKNOWN',
    desc: 'Standalone unknown license identifier',
  },

  // 8. Syntax & Malformed Expressions
  { expr: '', pkgId: 'test-pkg@1.0.0', expected: 'UNKNOWN', desc: 'Empty expression' },
  { expr: '   ', pkgId: 'test-pkg@1.0.0', expected: 'UNKNOWN', desc: 'Whitespace only' },
  { expr: '(MIT OR)', pkgId: 'test-pkg@1.0.0', expected: 'UNKNOWN', desc: 'Dangling operator in parentheses' },
  { expr: 'MIT AND', pkgId: 'test-pkg@1.0.0', expected: 'UNKNOWN', desc: 'Dangling binary operator' },
  { expr: '((MIT)', pkgId: 'test-pkg@1.0.0', expected: 'UNKNOWN', desc: 'Mismatched parentheses' },
  { expr: 'MIT Apache-2.0', pkgId: 'test-pkg@1.0.0', expected: 'UNKNOWN', desc: 'Missing operator between identifiers' },
];

console.log('Running SPDX Unit Tests (Phase 7)...');
let passed = 0;
let failed = 0;

for (const tc of testCases) {
  const result = evaluateLicenseExpression(tc.expr, tc.pkgId, individualExceptions);
  if (result.status === tc.expected) {
    console.log(`  ✓ [${result.status.padEnd(7)}] "${tc.expr}" => ${tc.desc}`);
    passed++;
  } else {
    console.error(`  ✗ FAIL: "${tc.expr}" => Expected ${tc.expected}, got ${result.status} (${result.reason})`);
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

console.log(`\nSPDX Unit Tests Finished: ${passed} passed, ${failed} failed.`);
if (failed > 0) {
  process.exit(1);
}
