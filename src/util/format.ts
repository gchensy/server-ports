import type { Forward } from "../api/types";

const UNITS = ["B", "KB", "MB", "GB", "TB"];

/** 1536 -> "1.5 KB". */
export function formatBytes(bytes: number): string {
  let value = Math.max(0, bytes);
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit++;
  }
  const digits = unit === 0 || value >= 100 ? 0 : value >= 10 ? 1 : 2;
  return `${value.toFixed(digits)} ${UNITS[unit]}`;
}

export function formatRate(bytesPerSecond: number): string {
  return `${formatBytes(bytesPerSecond)}/s`;
}

/** The address to type into a client. An explicit IP: "localhost" may resolve to ::1 first. */
export function localAddress(forward: Pick<Forward, "bindHost" | "localPort">): string {
  const host = forward.bindHost === "0.0.0.0" || forward.bindHost === "::" ? "127.0.0.1" : forward.bindHost;
  return `${host}:${forward.localPort}`;
}

export function remoteAddress(forward: Pick<Forward, "remoteHost" | "remotePort">): string {
  return `${forward.remoteHost}:${forward.remotePort}`;
}

/** Copies text; falls back to the old selection trick where the async clipboard is not allowed. */
export async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    const area = document.createElement("textarea");
    area.value = text;
    area.setAttribute("readonly", "");
    area.style.position = "fixed";
    area.style.opacity = "0";
    document.body.appendChild(area);
    area.select();
    try {
      return document.execCommand("copy");
    } finally {
      area.remove();
    }
  }
}
