import { AnimatePresence, motion } from "motion/react";
import { Laptop, Pencil, Power, PowerOff, Server, Trash, TriangleAlert } from "lucide-react";
import { useState } from "react";
import type { ConnState, Profile, ProfileRuntime } from "../api/types";
import { isActive } from "../state/AppStore";
import { useAppState, useI18n, useProfile, useStore } from "../state/services";
import { useNow } from "../state/useNow";
import { cx } from "../util/cx";
import { CopyLogButton, LogPanel } from "./LogPanel";
import { ForwardRow } from "./ForwardRow";
import { StatusDot } from "./StatusDot";
import { Button, IconButton, Tabs } from "./ui";

/** Errors that the user fixes by changing the server's settings. */
const FIXED_BY_EDITING = ["keyFile", "noListeners", "auth", "agent", "negotiation"];

export function Detail({ onEdit, onDelete }: { onEdit: (id: string) => void; onDelete: (id: string) => void }) {
  const selectedId = useAppState((state) => state.selectedId);
  const profile = useProfile(selectedId);
  return (
    <AnimatePresence mode="wait" initial={false}>
      {profile && (
        <motion.section
          key={profile.id}
          className="detail"
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, transition: { duration: 0.1 } }}
          transition={{ duration: 0.2, ease: "easeOut" }}
        >
          <ServerView profile={profile} onEdit={() => onEdit(profile.id)} onDelete={() => onDelete(profile.id)} />
        </motion.section>
      )}
    </AnimatePresence>
  );
}

function ServerView({ profile, onEdit, onDelete }: { profile: Profile; onEdit: () => void; onDelete: () => void }) {
  const i18n = useI18n();
  const { t } = i18n;
  const store = useStore();
  const runtime = useAppState((state) => state.runtimes[profile.id]);
  const active = isActive(runtime);
  const connected = runtime?.conn.state === "connected";
  const [tab, setTab] = useState<"ports" | "activity">("ports");

  return (
    <>
      <header className="detail__head">
        <div className="detail__title">
          <h1>{profile.name}</h1>
          <p className="detail__meta">
            <code>
              {profile.username}@{profile.host}
              {profile.port !== 22 && `:${profile.port}`}
            </code>
          </p>
        </div>
        <div className="detail__actions">
          <Button
            variant={active ? "secondary" : "primary"}
            icon={active ? <PowerOff size={16} /> : <Power size={16} />}
            onClick={() => void (active ? store.disconnect(profile.id) : store.connect(profile.id))}
          >
            {active ? t("detail.disconnect") : t("detail.connect")}
          </Button>
          <IconButton label={t("detail.edit")} onClick={onEdit}>
            <Pencil size={16} />
          </IconButton>
          <IconButton label={t("detail.delete")} danger onClick={onDelete}>
            <Trash size={16} />
          </IconButton>
        </div>
      </header>

      <Tunnel profile={profile} runtime={runtime} />
      <AnimatePresence initial={false}>
        <Problem key={`${profile.id}-${runtime?.conn.state}`} profile={profile} runtime={runtime} onEdit={onEdit} />
      </AnimatePresence>

      <Tabs
        value={tab}
        onChange={setTab}
        label={profile.name}
        options={[
          { value: "ports", label: t("detail.mappings") },
          { value: "activity", label: t("detail.log") },
        ]}
        actions={tab === "activity" ? <CopyLogButton profileId={profile.id} /> : undefined}
      >
        {tab === "ports" ? (
          <ul className="mappings__list">
            {profile.forwards.map((forward) => (
              <ForwardRow
                key={forward.id}
                forward={forward}
                runtime={runtime?.forwards.find((f) => f.id === forward.id)}
                connected={connected}
              />
            ))}
          </ul>
        ) : (
          <LogPanel profileId={profile.id} />
        )}
      </Tabs>
    </>
  );
}

export function authSummary(profile: Profile, t: ReturnType<typeof useI18n>["t"]): string {
  switch (profile.auth.type) {
    case "password":
      return t("detail.auth.password");
    case "key":
      return t("detail.auth.key", { path: profile.auth.path });
    case "agent":
      return t("detail.auth.agent");
  }
}

/** This computer, a line that shows what the connection is doing, and the server. */
function Tunnel({ profile, runtime }: { profile: Profile; runtime: ProfileRuntime | undefined }) {
  const { t, duration } = useI18n();
  const conn: ConnState = runtime?.conn ?? { state: "idle" };
  const ticking = conn.state === "connected" || conn.state === "reconnecting";
  const now = useNow(1000, ticking);

  let status: string;
  switch (conn.state) {
    case "connected":
      status = t("status.connectedFor", { time: duration(now - (runtime?.connectedAt ?? now)) });
      break;
    case "reconnecting":
      status = `${t("status.reconnecting")} · ${t("status.retryIn", { time: duration(Math.max(0, conn.retryAt - now)) })} · ${t("status.attempt", { n: conn.attempt })}`;
      break;
    default:
      status = t(`status.${conn.state}`);
  }

  return (
    <div className={cx("tunnel", `tunnel--${conn.state}`)}>
      <Laptop size={22} className="tunnel__icon" />
      <span className="tunnel__pipe" aria-hidden="true" />
      <Server size={22} className="tunnel__icon" />
      <span className="tunnel__name">{t("detail.thisComputer")}</span>
      <span className="tunnel__status" role="status">
        <StatusDot state={conn.state} />
        {status}
      </span>
      <span className="tunnel__name">{profile.host}</span>
    </div>
  );
}

/** Why the connection is down, and what can be done about it. */
function Problem({
  profile,
  runtime,
  onEdit,
}: {
  profile: Profile;
  runtime: ProfileRuntime | undefined;
  onEdit: () => void;
}) {
  const i18n = useI18n();
  const store = useStore();
  const conn = runtime?.conn;
  if (conn?.state !== "failed" && conn?.state !== "reconnecting") return null;
  const { title, hint } = i18n.describe(conn.error);
  const code = conn.error.code;

  return (
    <motion.div
      className={cx("banner", `banner--${conn.state}`)}
      role="alert"
      initial={{ opacity: 0, height: 0, marginBottom: 0 }}
      animate={{ opacity: 1, height: "auto", marginBottom: 16 }}
      exit={{ opacity: 0, height: 0, marginBottom: 0 }}
      transition={{ duration: 0.22, ease: "easeOut" }}
    >
      <div className="banner__inner">
        <TriangleAlert size={20} className="banner__icon" />
        <div className="banner__text">
          <strong>{title}</strong>
          {hint && <p>{hint}</p>}
          <code>{conn.error.detail}</code>
        </div>
        <div className="banner__actions">
          {conn.state === "reconnecting" && (
            <Button size="sm" onClick={() => void store.retryNow(profile.id)}>
              {i18n.t("detail.retryNow")}
            </Button>
          )}
          {code === "hostKeyChanged" && (
            <Button size="sm" variant="danger" onClick={() => void store.trustNewHostKey(profile.id)}>
              {i18n.t("action.forgetHostKey")}
            </Button>
          )}
          {conn.state === "failed" && FIXED_BY_EDITING.includes(code) && (
            <Button size="sm" onClick={onEdit}>
              {i18n.t("action.editServer")}
            </Button>
          )}
        </div>
      </div>
    </motion.div>
  );
}
