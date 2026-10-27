import { Copy } from "lucide-react";
import { useEffect, useRef } from "react";
import type { ProfileId } from "../api/types";
import { useAppState, useI18n, useStore } from "../state/services";
import { copyText } from "../util/format";
import { IconButton } from "./ui";

const NO_LINES: readonly never[] = [];

function useLines(profileId: ProfileId) {
  return useAppState((state) => state.logs[profileId] ?? NO_LINES);
}

/** What the engine reports about one server, newest at the bottom. */
export function LogPanel({ profileId }: { profileId: ProfileId }) {
  const i18n = useI18n();
  const lines = useLines(profileId);
  const body = useRef<HTMLDivElement>(null);
  const following = useRef(true);

  useEffect(() => {
    const element = body.current;
    if (element && following.current) element.scrollTop = element.scrollHeight;
  }, [lines, profileId]);

  return (
    <div
      className="log__body"
      ref={body}
      onScroll={(event) => {
        const el = event.currentTarget;
        following.current = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
      }}
    >
      {lines.length === 0 ? (
        <p className="log__empty">{i18n.t("detail.logEmpty")}</p>
      ) : (
        lines.map((line) => (
          <div key={line.seq} className={`log__line log__line--${line.level}`}>
            <time>{i18n.clock(line.ts)}</time>
            <span>{line.message}</span>
          </div>
        ))
      )}
    </div>
  );
}

/** Puts the whole log on the clipboard. */
export function CopyLogButton({ profileId }: { profileId: ProfileId }) {
  const i18n = useI18n();
  const store = useStore();
  const lines = useLines(profileId);

  const copy = async () => {
    const text = lines.map((l) => `${i18n.clock(l.ts)} ${l.level.toUpperCase()} ${l.message}`).join("\n");
    if (await copyText(text)) store.toast(i18n.t("toast.copied"));
  };

  return (
    <IconButton label={i18n.t("detail.copyLog")} onClick={() => void copy()} disabled={lines.length === 0}>
      <Copy size={15} />
    </IconButton>
  );
}
