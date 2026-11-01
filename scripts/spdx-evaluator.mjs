/**
 * SPDX License Expression Parser & Compliance Evaluator for Cuddle (Phase 7)
 *
 * Implements formal SPDX 3.0 recursive descent parsing:
 *   Expression  ::= OrExpr
 *   OrExpr      ::= AndExpr ( 'OR' AndExpr )*
 *   AndExpr     ::= WithExpr ( 'AND' WithExpr )*
 *   WithExpr    ::= SimpleLicense ( 'WITH' ExceptionIdent )? | '(' Expression ')'
 *   SimpleLicense ::= LicenseIdent ( '+' )?
 *
 * Evaluation Statuses:
 *   PASS    : Unconditionally allowed under Cuddle permissive policy.
 *   REVIEW  : Conditionally approved individual exception (e.g. build-time tool).
 *   FAIL    : Disallowed copyleft / restricted license.
 *   UNKNOWN : Unrecognized license identifier, unapproved exception, empty expression, or syntax error.
 */

// Permissive licenses accepted unconditionally
export const PERMISSIVE_LICENSES = new Set([
  'MIT',
  'Apache-2.0',
  'ISC',
  'BSD-2-Clause',
  'BSD-3-Clause',
  '0BSD',
  'BlueOak-1.0.0',
  'CC0-1.0',
  'MIT-0',
  'Unlicense',
  'BSL-1.0',
  'Zlib',
  'Unicode-3.0',
  'CDLA-Permissive-2.0',
  'PostgreSQL',
  'X11',
  'Python-2.0',
  'Artistic-2.0',
]);

// Known restrictive copyleft licenses recognized by SPDX
export const KNOWN_RESTRICTIVE_LICENSES = new Set([
  'GPL-1.0-only',
  'GPL-1.0-or-later',
  'GPL-2.0-only',
  'GPL-2.0-or-later',
  'GPL-3.0-only',
  'GPL-3.0-or-later',
  'AGPL-1.0-only',
  'AGPL-1.0-or-later',
  'AGPL-3.0-only',
  'AGPL-3.0-or-later',
  'LGPL-2.0-only',
  'LGPL-2.0-or-later',
  'LGPL-2.1-only',
  'LGPL-2.1-or-later',
  'LGPL-3.0-only',
  'LGPL-3.0-or-later',
  'MPL-1.0',
  'MPL-1.1',
  'MPL-2.0',
  'SSPL-1.0',
  'EUPL-1.1',
  'EUPL-1.2',
  'EPL-1.0',
  'EPL-2.0',
  'CC-BY-SA-4.0',
  'CC-BY-NC-4.0',
]);

// Known formal SPDX license exception identifiers
export const KNOWN_SPDX_EXCEPTIONS = new Set([
  'LLVM-exception',
  'Classpath-exception-2.0',
  'Autoconf-exception-2.0',
  'Autoconf-exception-3.0',
  'Bison-exception-2.2',
  'Bootloader-exception',
  'CLISP-exception-2.0',
  'DigiRule-FIRMWARE-exception',
  'FLOSS-exception',
  'GPL-3.0-linking-exception',
  'GPL-3.0-linking-source-exception',
  'GPL-CC-1.0',
  'LGPL-3.0-linking-exception',
  'Libtool-exception',
  'Linux-syscall-note',
  'OCCT-exception-1.0',
  'OpenJDK-assembly-exception-1.0',
  'Qt-GPL-exception-1.0',
  'Qt-LGPL-exception-1.1',
  'Qwt-exception-1.0',
  'Swift-exception',
  'u-boot-exception-2.0',
  'Universal-FOSS-exception-1.0',
  'WxWindows-exception-3.1',
]);

// Approved base license + exception combinations under Cuddle's compliance policy
export const APPROVED_WITH_COMBINATIONS = new Map([
  ['Apache-2.0', new Set(['LLVM-exception'])],
]);

/**
 * Tokenizes an SPDX expression string into an array of tokens.
 * Strictly adheres to SPDX 3.0 keywords: AND, OR, WITH.
 * Does NOT accept non-standard symbols like || or &&.
 */
export function tokenizeSpdx(exprStr) {
  if (typeof exprStr !== 'string') return null;
  const trimmed = exprStr.trim();
  if (trimmed === '') return null;

  const tokens = [];
  let i = 0;
  while (i < trimmed.length) {
    const ch = trimmed[i];

    if (/\s/.test(ch)) {
      i++;
      continue;
    }

    if (ch === '(') {
      tokens.push({ type: 'LPAREN', value: '(' });
      i++;
      continue;
    }

    if (ch === ')') {
      tokens.push({ type: 'RPAREN', value: ')' });
      i++;
      continue;
    }

    // Identifiers or operators
    let start = i;
    while (i < trimmed.length && !/\s|\(|\)/.test(trimmed[i])) {
      i++;
    }
    const word = trimmed.slice(start, i);

    // Reject non-SPDX symbols (like ||, &&, |, &)
    if (word === '||' || word === '&&' || word === '|' || word === '&') {
      return null; // Syntax error
    }

    if (word === 'OR') {
      tokens.push({ type: 'OR', value: 'OR' });
    } else if (word === 'AND') {
      tokens.push({ type: 'AND', value: 'AND' });
    } else if (word === 'WITH') {
      tokens.push({ type: 'WITH', value: 'WITH' });
    } else {
      // Must be a valid identifier format
      if (!/^[A-Za-z0-9_.\-+]+$/.test(word)) {
        return null;
      }
      tokens.push({ type: 'IDENT', value: word });
    }
  }

  return tokens;
}

/**
 * Parses tokens into an AST using recursive descent.
 * Strictly enforces that WITH can ONLY follow a SimpleLicense (IDENT),
 * NOT a parenthesized or compound expression.
 */
export function parseSpdxTokens(tokens) {
  if (!tokens || tokens.length === 0) return null;
  let pos = 0;

  function peek() {
    return tokens[pos] || null;
  }

  function consume(expectedType) {
    const tok = peek();
    if (!tok || (expectedType && tok.type !== expectedType)) {
      return null;
    }
    pos++;
    return tok;
  }

  function parseExpression() {
    return parseOr();
  }

  function parseOr() {
    let left = parseAnd();
    if (!left) return null;

    while (peek() && peek().type === 'OR') {
      consume('OR');
      const right = parseAnd();
      if (!right) return null;
      left = { type: 'OR', left, right };
    }
    return left;
  }

  function parseAnd() {
    let left = parseWith();
    if (!left) return null;

    while (peek() && peek().type === 'AND') {
      consume('AND');
      const right = parseWith();
      if (!right) return null;
      left = { type: 'AND', left, right };
    }
    return left;
  }

  function parseWith() {
    const tok = peek();
    if (!tok) return null;

    // If it starts with '(', parse as a parenthesized Expression
    if (tok.type === 'LPAREN') {
      consume('LPAREN');
      const inner = parseExpression();
      if (!inner) return null;
      if (!consume('RPAREN')) return null;

      // In formal SPDX grammar, WITH CANNOT follow a parenthesized expression!
      // e.g. (MIT OR Apache-2.0) WITH LLVM-exception is invalid!
      if (peek() && peek().type === 'WITH') {
        return null; // Syntax error: WITH on compound expression
      }
      return inner;
    }

    // Otherwise, must be a SimpleLicense IDENT
    if (tok.type === 'IDENT') {
      consume('IDENT');
      const licenseNode = { type: 'LICENSE', id: tok.value };

      // Check if followed by WITH
      if (peek() && peek().type === 'WITH') {
        consume('WITH');
        const excToken = consume('IDENT');
        if (!excToken) return null;
        return { type: 'WITH', license: licenseNode, exception: excToken.value };
      }

      return licenseNode;
    }

    return null;
  }

  const ast = parseExpression();
  if (!ast || pos !== tokens.length) {
    // Extra unconsumed tokens -> syntax error
    return null;
  }
  return ast;
}

/**
 * Validates ALL identifiers and WITH exception combinations in the AST.
 * This runs BEFORE policy evaluation so that any unknown identifier
 * anywhere in the expression (even in an OR branch) immediately yields UNKNOWN.
 *
 * @param {object} ast
 * @param {string} pkgId
 * @param {Map<string, { license: string, reason: string }>} individualExceptions
 * @returns {{ valid: boolean, status?: 'UNKNOWN', reason?: string }}
 */
export function validateAstIdentifiers(ast, pkgId, individualExceptions) {
  if (!ast) {
    return { valid: false, status: 'UNKNOWN', reason: 'Empty or invalid AST node' };
  }

  if (ast.type === 'LICENSE') {
    const id = ast.id;
    // Check if approved individual exception for this package
    if (individualExceptions && individualExceptions.has(pkgId)) {
      const exc = individualExceptions.get(pkgId);
      if (exc.license === id) {
        return { valid: true };
      }
    }

    // Check permissive list
    if (PERMISSIVE_LICENSES.has(id)) {
      return { valid: true };
    }

    // Check known restrictive list
    if (KNOWN_RESTRICTIVE_LICENSES.has(id)) {
      return { valid: true };
    }

    // Check standard prefix/suffix patterns like GPL-2.0+ or Apache-2.0+
    if (id.endsWith('+')) {
      const base = id.slice(0, -1);
      if (PERMISSIVE_LICENSES.has(base) || KNOWN_RESTRICTIVE_LICENSES.has(base)) {
        return { valid: true };
      }
    }

    return { valid: false, status: 'UNKNOWN', reason: `Unrecognized license identifier: "${id}"` };
  }

  if (ast.type === 'WITH') {
    const licVal = validateAstIdentifiers(ast.license, pkgId, individualExceptions);
    if (!licVal.valid) return licVal;

    const exc = ast.exception;
    if (!KNOWN_SPDX_EXCEPTIONS.has(exc)) {
      return { valid: false, status: 'UNKNOWN', reason: `Unrecognized SPDX exception identifier: "${exc}"` };
    }

    // Validate license + exception combination
    const baseId = ast.license.id;
    const allowedForBase = APPROVED_WITH_COMBINATIONS.get(baseId);
    if (!allowedForBase || !allowedForBase.has(exc)) {
      return {
        valid: false,
        status: 'UNKNOWN',
        reason: `Unapproved license exception combination: "${baseId} WITH ${exc}"`,
      };
    }

    return { valid: true };
  }

  if (ast.type === 'OR' || ast.type === 'AND') {
    const leftVal = validateAstIdentifiers(ast.left, pkgId, individualExceptions);
    if (!leftVal.valid) return leftVal;
    const rightVal = validateAstIdentifiers(ast.right, pkgId, individualExceptions);
    if (!rightVal.valid) return rightVal;
    return { valid: true };
  }

  return { valid: false, status: 'UNKNOWN', reason: `Invalid AST node type: ${ast.type}` };
}

/**
 * Evaluates an already validated SPDX AST against Cuddle's compliance policy.
 *
 * @param {object} ast
 * @param {string} pkgId
 * @param {Map<string, { license: string, reason: string }>} individualExceptions
 * @returns {{ status: 'PASS' | 'REVIEW' | 'FAIL' | 'UNKNOWN', reason?: string }}
 */
export function evaluateAstPolicy(ast, pkgId, individualExceptions) {
  if (ast.type === 'LICENSE') {
    const id = ast.id;

    // Check individual exception
    if (individualExceptions && individualExceptions.has(pkgId)) {
      const exc = individualExceptions.get(pkgId);
      if (exc.license === id) {
        return { status: 'REVIEW', approved: true, reason: `Individually audited exception: ${exc.reason}` };
      }
    }

    // Check permissive
    if (PERMISSIVE_LICENSES.has(id)) {
      return { status: 'PASS' };
    }

    // Check known restrictive
    if (KNOWN_RESTRICTIVE_LICENSES.has(id)) {
      if (id === 'MPL-2.0' || id === 'MPL-1.1' || id === 'MPL-1.0') {
        return {
          status: 'REVIEW',
          approved: false,
          reason: `Weak copyleft license requiring individual review (unapproved exception): ${id}`,
        };
      }
      return { status: 'FAIL', reason: `Disallowed copyleft/restricted license: ${id}` };
    }

    return { status: 'UNKNOWN', reason: `Unrecognized license identifier: ${id}` };
  }

  if (ast.type === 'WITH') {
    // Approved combinations (e.g. Apache-2.0 WITH LLVM-exception) are PASS
    const baseEval = evaluateAstPolicy(ast.license, pkgId, individualExceptions);
    if (baseEval.status === 'PASS') {
      return { status: 'PASS' };
    }
    return baseEval;
  }

  if (ast.type === 'OR') {
    const left = evaluateAstPolicy(ast.left, pkgId, individualExceptions);
    const right = evaluateAstPolicy(ast.right, pkgId, individualExceptions);

    // If either option is PASS, the licensee may choose the PASS option
    if (left.status === 'PASS' || right.status === 'PASS') {
      return { status: 'PASS' };
    }

    // If either is REVIEW, choice allows REVIEW
    if (left.status === 'REVIEW' || right.status === 'REVIEW') {
      return left.status === 'REVIEW' ? left : right;
    }

    return { status: 'FAIL', reason: `No allowed option in OR: ${left.reason} AND ${right.reason}` };
  }

  if (ast.type === 'AND') {
    const left = evaluateAstPolicy(ast.left, pkgId, individualExceptions);
    const right = evaluateAstPolicy(ast.right, pkgId, individualExceptions);

    // In AND, all terms apply. Any FAIL is fatal.
    if (left.status === 'FAIL') return left;
    if (right.status === 'FAIL') return right;
    if (left.status === 'REVIEW' || right.status === 'REVIEW') {
      return { status: 'REVIEW', reason: `${left.reason || ''} ${right.reason || ''}`.trim() };
    }
    return { status: 'PASS' };
  }

  return { status: 'UNKNOWN', reason: `Unknown AST node type: ${ast.type}` };
}

/**
 * Main evaluation entrypoint for an SPDX license string.
 *
 * Enforces two-step pipeline:
 *   1. Tokenization & AST Parsing (rejects invalid operators, syntax, and WITH scopes)
 *   2. Global AST Identifier & Exception Validation (rejects unknown licenses anywhere in expr)
 *   3. Logical Policy Evaluation (applies AND / OR / WITH rules)
 */
export function evaluateLicenseExpression(exprStr, pkgId, individualExceptions) {
  if (!exprStr || typeof exprStr !== 'string' || exprStr.trim() === '') {
    return { status: 'UNKNOWN', reason: 'Missing or empty license string' };
  }

  const tokens = tokenizeSpdx(exprStr);
  if (!tokens || tokens.length === 0) {
    return { status: 'UNKNOWN', reason: `Syntax error or invalid operators in license expression: "${exprStr}"` };
  }

  const ast = parseSpdxTokens(tokens);
  if (!ast) {
    return { status: 'UNKNOWN', reason: `Grammar syntax error in SPDX expression: "${exprStr}"` };
  }

  // Step 1: Validate all identifiers & combinations
  const validation = validateAstIdentifiers(ast, pkgId, individualExceptions);
  if (!validation.valid) {
    return { status: 'UNKNOWN', reason: validation.reason };
  }

  // Step 2: Logical policy evaluation
  return evaluateAstPolicy(ast, pkgId, individualExceptions);
}
