import { AnimatePresence, motion } from "motion/react";
import { Plus, Settings } from "lucide-react";
import type { Profile } from "../api/types";
import { isActive } from "../state/AppStore";
import { useAppState, useI18n, useStore } from "../state/services";
import { cx } from "../util/cx";
import { Logo } from "./Logo";
import { spring } from "./motion";
import { StatusDot } from "./StatusDot";
import { Button, Switch } from "./ui";

export function Sidebar({ onNew, onSettings }: { onNew: () => void; onSettings: () => void }) {
  const { t } = useI18n();
  const profiles = useAppState((state) => state.profiles);
  return (
    <aside className="sidebar">
      <header className="sidebar__brand">
        <Logo />
        <span>{t("app.name")}</span>
      </header>
      <Button variant="primary" icon={<Plus size={16} />} className="sidebar__new" onClick={onNew}>
        {t("sidebar.new")}
      </Button>
      {profiles.length > 0 && <div className="sidebar__label">{t("sidebar.servers")}</div>}
      <nav className="sidebar__list" aria-label={t("sidebar.servers")}>
        <AnimatePresence initial={false}>
          {profiles.map((profile) => (
            <ProfileCard key={profile.id} profile={profile} />
          ))}
        </AnimatePresence>
      </nav>
      <footer className="sidebar__footer">
        <button type="button" className="sidebar__action" onClick={onSettings}>
          <Settings size={16} />
          {t("sidebar.settings")}
        </button>
      </footer>
    </aside>
  );
}

function ProfileCard({ profile }: { profile: Profile }) {
  const i18n = useI18n();
  const store = useStore();
  const runtime = useAppState((state) => state.runtimes[profile.id]);
  const selected = useAppState((state) => state.selectedId === profile.id);
  const state = runtime?.conn.state ?? "idle";
  const busy = state === "connecting" || state === "reconnecting";
  return (
    <motion.div
      layout="position"
      className={cx("card", selected && "is-selected")}
      initial={{ opacity: 0, x: -14 }}
      animate={{ opacity: 1, x: 0 }}
      exit={{ opacity: 0, x: -14, transition: { duration: 0.14 } }}
      transition={spring}
    >
      {selected && <motion.span layoutId="card-selection" className="card__selection" transition={spring} />}
      <button
        type="button"
        className="card__main"
        aria-current={selected ? "true" : undefined}
        onClick={() => store.select(profile.id)}
      >
        <StatusDot state={state} />
        <span className="card__text">
          <span className="card__name">{profile.name}</span>
          <span className="card__sub">
            {profile.username}@{profile.host}
          </span>
        </span>
      </button>
      <Switch
        checked={isActive(runtime)}
        busy={busy}
        label={i18n.t(isActive(runtime) ? "sidebar.disconnect" : "sidebar.connect", { name: profile.name })}
        onChange={(on) => void (on ? store.connect(profile.id) : store.disconnect(profile.id))}
      />
    </motion.div>
  );
}
