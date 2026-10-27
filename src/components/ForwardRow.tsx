import { ArrowDown, ArrowRight, ArrowUp, Check, Copy, ExternalLink } from "lucide-react";
import { useState } from "react";
import type { Forward, ForwardRuntime } from "../api/types";
import { useI18n, useSamples, useStore } from "../state/services";
import { cx } from "../util/cx";
import { copyText, formatBytes, formatRate, localAddress, localUrl, remoteAddress } from "../util/format";
import { Sparkline } from "./Sparkline";
import { IconButton } from "./ui";

/** One port mapping: where it listens, where it leads, and what flows through it. */
export function ForwardRow({
  forward,
  runtime,
  connected,
}: {
  forward: Forward;
  runtime: ForwardRuntime | undefined;
  connected: boolean;
}) {
  const i18n = useI18n();
  const { t } = i18n;
  const store = useStore();
  const samples = useSamples(forward.id);
  const [copied, setCopied] = useState(false);

  const state = runtime?.state.state ?? "pending";
  const listening = connected && state === "listening";
  const last = samples[samples.length - 1];

  const copy = async () => {
    if (!(await copyText(localAddress(forward)))) return;
    setCopied(true);
    store.toast(t("toast.copied"));
    window.setTimeout(() => setCopied(false), 1400);
  };

  return (
    <li className={cx("mapping", `mapping--${listening ? "listening" : state}`)}>
      <div className="mapping__route">
        <span className="mapping__dot" aria-hidden="true" />
        <code className="mapping__local">{localAddress(forward)}</code>
        <ArrowRight size={14} className="mapping__arrow" aria-hidden="true" />
        <code className="mapping__remote">{remoteAddress(forward)}</code>
        {forward.label && <span className="chip">{forward.label}</span>}
        <span className="mapping__spacer" />
        <span className="mapping__tools">
          <IconButton
            label={t("forward.openLink")}
            onClick={() => void store.openLink(localUrl(forward))}
            disabled={!listening}
          >
            <ExternalLink size={15} />
          </IconButton>
          <IconButton label={t("forward.copy")} onClick={() => void copy()}>
            {copied ? <Check size={15} /> : <Copy size={15} />}
          </IconButton>
        </span>
      </div>

      {runtime?.state.state === "failed" && (
        <p className="mapping__problem">
          <strong>{i18n.errorTitle(runtime.state.error.code)}</strong>
          <span>{runtime.state.error.detail}</span>
        </p>
      )}

      {listening && runtime && (
        <div className="mapping__stats">
          <Sparkline samples={samples} live={(last?.up ?? 0) + (last?.down ?? 0) > 0} />
          <dl className="mapping__numbers">
            <div title={t("forward.up")}>
              <dt>
                <ArrowUp size={12} />
              </dt>
              <dd>
                <span>{formatBytes(runtime.bytesUp)}</span>
                <small>{formatRate(last?.up ?? 0)}</small>
              </dd>
            </div>
            <div title={t("forward.down")}>
              <dt>
                <ArrowDown size={12} />
              </dt>
              <dd>
                <span>{formatBytes(runtime.bytesDown)}</span>
                <small>{formatRate(last?.down ?? 0)}</small>
              </dd>
            </div>
            <div title={t("forward.connections")}>
              <dd>
                {t("forward.open", { n: runtime.active })}
                <small>{t("forward.total", { n: runtime.total })}</small>
              </dd>
            </div>
          </dl>
        </div>
      )}
    </li>
  );
}
