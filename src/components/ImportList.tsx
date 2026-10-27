import { useState } from "react";
import type { DiscoveredHost, Profile } from "../api/types";
import { useI18n, useStore } from "../state/services";
import { cx } from "../util/cx";
import { authSummary } from "./Detail";
import { Button } from "./ui";

/** A server the app can take over as it is: not one of ours yet, and not behind a proxy. */
const importable = (host: DiscoveredHost) => !host.alreadyAdded && host.proxy === null;

/** The servers that name their ports are the ones the user set up to be forwarded: they start out picked. */
const preselected = (host: DiscoveredHost) => importable(host) && host.profile.forwards.length > 0;

/**
 * The servers of the user's SSH config, to pick from, with the buttons that end the dialog around
 * it. They are copied into the app and the file is only read. `onFinish` gets the picked servers
 * that are not saved yet: they have no ports to forward, or the core refused them, and the form
 * has to complete them. `hosts` is null while the file is being read.
 */
export function ImportList({
  hosts,
  onClose,
  onFinish,
}: {
  hosts: DiscoveredHost[] | null;
  onClose: () => void;
  onFinish: (unsaved: Profile[]) => void;
}) {
  const i18n = useI18n();
  const { t } = i18n;
  const store = useStore();
  // Only what the user changed from the starting choice, so a list that arrives late needs no set-up.
  const [flipped, setFlipped] = useState<ReadonlySet<number>>(new Set());
  const [busy, setBusy] = useState(false);

  const picked = (host: DiscoveredHost, index: number) => importable(host) && preselected(host) !== flipped.has(index);
  const chosen = (hosts ?? []).filter(picked);

  const toggle = (index: number) =>
    setFlipped((current) => {
      const next = new Set(current);
      if (!next.delete(index)) next.add(index);
      return next;
    });

  const confirm = async () => {
    setBusy(true);
    const profiles = chosen.map((host) => host.profile);
    const refused = await store.importProfiles(profiles.filter((p) => p.forwards.length > 0));
    onFinish([...refused, ...profiles.filter((p) => p.forwards.length === 0)]);
  };

  return (
    <>
      <div className="modal__scroll">
        <section className="form-section">
          <p className="form-note">{t("import.intro")}</p>
          {hosts?.length === 0 && <p className="import-none">{t("import.empty")}</p>}
          {hosts && hosts.length > 0 && (
            <ul className="import-list" aria-label={t("import.title")}>
              {hosts.map((host, index) => {
                const { profile } = host;
                const on = picked(host, index);
                return (
                  <li key={index}>
                    <label className={cx("import-row", on && "is-on", !importable(host) && "is-off")}>
                      <input
                        type="checkbox"
                        checked={on}
                        disabled={!importable(host)}
                        onChange={() => toggle(index)}
                      />
                      <span className="import-row__text">
                        <span className="import-row__head">
                          <span className="import-row__name">{profile.name}</span>
                          {host.alreadyAdded && <span className="chip chip--muted">{t("import.added")}</span>}
                          {host.proxy && (
                            <span className="chip chip--warn">{t("import.unsupported", { kind: host.proxy })}</span>
                          )}
                        </span>
                        <span className="import-row__meta">
                          <code>
                            {profile.username}@{profile.host}:{profile.port}
                          </code>
                          <span>{authSummary(profile, t)}</span>
                          <span>
                            {profile.forwards.length > 0
                              ? i18n.plural("ports", profile.forwards.length)
                              : t("import.noPorts")}
                          </span>
                        </span>
                      </span>
                    </label>
                  </li>
                );
              })}
            </ul>
          )}
        </section>
      </div>
      <footer className="modal__footer">
        <p className="modal__footnote">{t("import.selected", { n: chosen.length })}</p>
        <Button onClick={onClose}>{t("common.cancel")}</Button>
        <Button variant="primary" disabled={busy || chosen.length === 0} onClick={() => void confirm()}>
          {t("import.confirm")}
        </Button>
      </footer>
    </>
  );
}
