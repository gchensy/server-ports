import { useEffect, useState } from "react";
import type { DiscoveredHost, Profile } from "../api/types";
import { useI18n, useServices } from "../state/services";
import { cx } from "../util/cx";
import { authSummary } from "./Detail";
import { Modal } from "./Modal";
import { Button } from "./ui";

/** A server the app can take over as it is: not one of ours yet, and not behind a proxy. */
const importable = (host: DiscoveredHost) => !host.alreadyAdded && host.proxy === null;

/**
 * The servers of the user's SSH config, to pick from. They are copied into the app and the file
 * is only read. `onFinish` gets the picked servers that are not saved yet: they have no ports to
 * forward, or the core refused them, and the form has to complete them.
 */
export function ImportDialog({ onClose, onFinish }: { onClose: () => void; onFinish: (unsaved: Profile[]) => void }) {
  const i18n = useI18n();
  const { t } = i18n;
  const { backend, store } = useServices();
  const [hosts, setHosts] = useState<DiscoveredHost[] | null>(null);
  const [chosen, setChosen] = useState<ReadonlySet<number>>(new Set());
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let live = true;
    backend.discoverSshHosts().then(
      (found) => {
        if (!live) return;
        setHosts(found);
        // The servers that name their ports are the ones the user set up to be forwarded.
        setChosen(new Set(found.flatMap((host, i) => (importable(host) && host.profile.forwards.length > 0 ? [i] : []))));
      },
      (raw) => {
        if (!live) return;
        store.reportError(raw);
        onClose();
      },
    );
    return () => {
      live = false;
    };
  }, [backend, store]);

  const toggle = (index: number) =>
    setChosen((current) => {
      const next = new Set(current);
      if (!next.delete(index)) next.add(index);
      return next;
    });

  const confirm = async () => {
    if (!hosts) return;
    setBusy(true);
    const picked = hosts.filter((_, i) => chosen.has(i)).map((host) => host.profile);
    const refused = await store.importProfiles(picked.filter((p) => p.forwards.length > 0));
    onFinish([...refused, ...picked.filter((p) => p.forwards.length === 0)]);
  };

  return (
    <Modal title={t("import.title")} onClose={onClose} wide>
      <div className="modal__form">
        <header className="modal__head">
          <h2>{t("import.title")}</h2>
        </header>
        <div className="modal__scroll">
          <section className="form-section">
            <p className="form-note">{t("import.intro")}</p>
            {hosts?.length === 0 && <p className="import-none">{t("import.empty")}</p>}
            {hosts && hosts.length > 0 && (
              <ul className="import-list" aria-label={t("import.title")}>
                {hosts.map((host, index) => {
                  const { profile } = host;
                  const on = chosen.has(index);
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
          <p className="modal__footnote">{t("import.selected", { n: chosen.size })}</p>
          <Button onClick={onClose}>{t("common.cancel")}</Button>
          <Button variant="primary" disabled={busy || chosen.size === 0} onClick={() => void confirm()}>
            {t("import.confirm")}
          </Button>
        </footer>
      </div>
    </Modal>
  );
}
