import { invoke } from "@tauri-apps/api/core";
import type { NotificationLevel } from "../types/NotificationLevel";

/**
 * Dispatches a native desktop notification to the host OS (COSMIC, GNOME, KDE, etc.)
 * via the backend FreeDesktop notification abstraction (Task 2-3).
 *
 * Gracefully handles headless/test environments and logs failures without throwing.
 */
export async function notifyUser(
  title: string,
  body: string,
  level: NotificationLevel = "info"
): Promise<boolean> {
  try {
    const delivered = await invoke<boolean>("notify_user", {
      title,
      body,
      level,
    });
    return delivered;
  } catch (err) {
    console.warn("[Notification] notifyUser failed or backend command unavailable:", err);
    return false;
  }
}

