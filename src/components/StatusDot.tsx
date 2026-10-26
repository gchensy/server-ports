import type { ConnState } from "../api/types";
import { cx } from "../util/cx";

/** The state of a server in one glance; the CSS animates it. */
export function StatusDot({ state, className }: { state: ConnState["state"]; className?: string }) {
  return <span className={cx("dot", `dot--${state}`, className)} aria-hidden="true" />;
}
