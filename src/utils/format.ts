/**
 * Formats a byte number into human-readable size (e.g. 1.2 KiB, 4.5 MiB).
 */
export function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KiB", "MiB", "GiB", "TiB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  const val = bytes / Math.pow(k, i);
  return `${val.toFixed(i === 0 ? 0 : 1)} ${sizes[i]}`;
}

/**
 * Formats epoch milliseconds into a concise readable timestamp.
 */
export function formatTimestamp(mtimeMs: number): string {
  if (!mtimeMs) return "-";
  const date = new Date(mtimeMs);
  return date.toLocaleString(undefined, {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}
