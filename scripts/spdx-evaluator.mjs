/**
 * SPDX License Expression Parser & Compliance Evaluator for Cuddle
 *
 * Implements standard SPDX recursive descent parsing:
 *   Expression ::= OrExpr
 *   OrExpr     ::= AndExpr ( 'OR' AndExpr )*
 *   AndExpr    ::= WithExpr ( 'AND' WithExpr )*
 *   WithExpr   ::= PrimaryExpr ( 'WITH' ExceptionIdent )?
 *   PrimaryExpr::= '(' Expression ')' | LicenseIdent
 *
 * Evaluation Statuses:
 *   PASS    : Unconditionally allowed under Cuddle permissive policy.
 *   REVIEW  : Conditionally approved individual exception (e.g. build-time tool).
 *   FAIL    : Disallowed copyleft / restricted license.
 *   UNKNOWN : Unrecognized license identifier, empty expression, or syntax error.
 */

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
]);

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

export const ALLOWED_EXCEPTIONS = new Set([
  'LLVM-exception',
]);

/**
 * Tokenizes an SPDX expression string into an array of tokens.
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

    if (word === 'OR' || word === '||') {
      tokens.push({ type: 'OR', value: 'OR' });
    } else if (word === 'AND' || word === '&&') {
      tokens.push({ type: 'AND', value: 'AND' });
    } else if (word === 'WITH') {
      tokens.push({ type: 'WITH', value: 'WITH' });
    } else {
      tokens.push({ type: 'IDENT', value: word });
    }
  }

  return tokens;
}

/**
 * Parses tokens into an AST using recursive descent.
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
    let left = parsePrimary();
    if (!left) return null;

    if (peek() && peek().type === 'WITH') {
      consume('WITH');
      const excToken = consume('IDENT');
      if (!excToken) return null;
      left = { type: 'WITH', license: left, exception: excToken.value };
    }
    return left;
  }

  function parsePrimary() {
    const tok = peek();
    if (!tok) return null;

    if (tok.type === 'LPAREN') {
      consume('LPAREN');
      const expr = parseExpression();
      if (!expr) return null;
      if (!consume('RPAREN')) return null;
      return expr;
    }

    if (tok.type === 'IDENT') {
      consume('IDENT');
      return { type: 'LICENSE', id: tok.value };
    }

    return null;
  }

  const ast = parseExpression();
  if (pos !== tokens.length) {
    // Unconsumed tokens left -> syntax error
    return null;
  }
  return ast;
}

/**
 * Evaluates an SPDX AST against Cuddle's compliance policy.
 *
 * @param {object} ast
 * @param {string} pkgId Package identifier, e.g. "lightningcss@1.33.0"
 * @param {Map<string, { license: string, reason: string }>} individualExceptions
 * @returns {{ status: 'PASS' | 'REVIEW' | 'FAIL' | 'UNKNOWN', reason?: string }}
 */
export function evaluateAst(ast, pkgId, individualExceptions) {
  if (!ast) {
    return { status: 'UNKNOWN', reason: 'Invalid or empty AST' };
  }

  if (ast.type === 'LICENSE') {
    const lic = ast.id;

    // 1. Check individual approved exception
    if (individualExceptions && individualExceptions.has(pkgId)) {
      const exc = individualExceptions.get(pkgId);
      if (exc.license === lic) {
        return { status: 'REVIEW', reason: `Individually audited exception: ${exc.reason}` };
      }
    }

    // 2. Check permissive whitelist
    if (PERMISSIVE_LICENSES.has(lic)) {
      return { status: 'PASS' };
    }

    // 3. Check known restrictive / copyleft
    if (KNOWN_RESTRICTIVE_LICENSES.has(lic) || lic.startsWith('GPL-') || lic.startsWith('AGPL-') || lic.startsWith('LGPL-') || lic.startsWith('MPL-')) {
      return { status: 'FAIL', reason: `Disallowed copyleft/restricted license: ${lic}` };
    }

    // 4. Unrecognized identifier
    return { status: 'UNKNOWN', reason: `Unrecognized license identifier: ${lic}` };
  }

  if (ast.type === 'WITH') {
    const baseEval = evaluateAst(ast.license, pkgId, individualExceptions);
    if (baseEval.status === 'FAIL' || baseEval.status === 'UNKNOWN') {
      return baseEval;
    }
    const exc = ast.exception;
    if (ALLOWED_EXCEPTIONS.has(exc)) {
      return baseEval;
    }
    return { status: 'UNKNOWN', reason: `Unrecognized license exception: WITH ${exc}` };
  }

  if (ast.type === 'OR') {
    const left = evaluateAst(ast.left, pkgId, individualExceptions);
    const right = evaluateAst(ast.right, pkgId, individualExceptions);

    // If either choice is PASS, the licensee may choose the PASS option!
    if (left.status === 'PASS' || right.status === 'PASS') {
      return { status: 'PASS' };
    }

    // If neither is PASS, but at least one is REVIEW, licensee chooses REVIEW
    if (left.status === 'REVIEW' || right.status === 'REVIEW') {
      return left.status === 'REVIEW' ? left : right;
    }

    // If either is UNKNOWN, cannot confirm compliance
    if (left.status === 'UNKNOWN' || right.status === 'UNKNOWN') {
      return { status: 'UNKNOWN', reason: `Unresolved choice: ${left.reason || ''} / ${right.reason || ''}` };
    }

    return { status: 'FAIL', reason: `No allowed option in OR: ${left.reason} AND ${right.reason}` };
  }

  if (ast.type === 'AND') {
    const left = evaluateAst(ast.left, pkgId, individualExceptions);
    const right = evaluateAst(ast.right, pkgId, individualExceptions);

    // In conjunction (AND), all terms apply! Any failure is fatal.
    if (left.status === 'FAIL') return left;
    if (right.status === 'FAIL') return right;
    if (left.status === 'UNKNOWN') return left;
    if (right.status === 'UNKNOWN') return right;

    if (left.status === 'REVIEW' || right.status === 'REVIEW') {
      return { status: 'REVIEW', reason: `${left.reason || ''} ${right.reason || ''}`.trim() };
    }

    return { status: 'PASS' };
  }

  return { status: 'UNKNOWN', reason: `Unknown AST node type: ${ast.type}` };
}

/**
 * Main evaluation entrypoint for an SPDX license string.
 */
export function evaluateLicenseExpression(exprStr, pkgId, individualExceptions) {
  if (!exprStr || typeof exprStr !== 'string' || exprStr.trim() === '') {
    return { status: 'UNKNOWN', reason: 'Missing or empty license string' };
  }

  const tokens = tokenizeSpdx(exprStr);
  if (!tokens || tokens.length === 0) {
    return { status: 'UNKNOWN', reason: 'Failed to tokenize license expression' };
  }

  const ast = parseSpdxTokens(tokens);
  if (!ast) {
    return { status: 'UNKNOWN', reason: `Syntax error in SPDX expression: "${exprStr}"` };
  }

  return evaluateAst(ast, pkgId, individualExceptions);
}

