import type { BackendError } from "./types";

export type AppError = BackendError | { code: "other"; detail: string };

/** A command that failed, with what the core said about it. */
export class CommandError extends Error {
  constructor(readonly error: AppError) {
    super("detail" in error ? error.detail : error.code);
    this.name = "CommandError";
  }
}

/** What a rejected `invoke` carries: our serialised `BackendError`, or a plain string. */
export function toCommandError(raw: unknown): CommandError {
  if (raw instanceof CommandError) return raw;
  if (typeof raw === "object" && raw !== null && "code" in raw) {
    return new CommandError(raw as BackendError);
  }
  return new CommandError({ code: "other", detail: String(raw) });
}
