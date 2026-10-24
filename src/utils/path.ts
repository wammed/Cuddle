/**
 * Safely encodes a UTF-8 path string into standard Base64 representation.
 * Handles non-ASCII characters (Japanese, Emoji, etc.) without throwing InvalidCharacterError.
 */
export function encodePathToBase64(pathStr: string): string {
  const bytes = new TextEncoder().encode(pathStr);
  let binary = "";
  const len = bytes.byteLength;
  for (let i = 0; i < len; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

/**
 * Decodes a Base64 string back into a UTF-8 string (lossy fallback for display).
 */
export function decodePathFromBase64(base64: string): string {
  try {
    const binary = atob(base64);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
      bytes[i] = binary.charCodeAt(i);
    }
    return new TextDecoder().decode(bytes);
  } catch {
    return base64;
  }
}
