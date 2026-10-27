import { motion, useAnimationControls } from "motion/react";
import { FolderOpen, KeyRound, Lock, Plus, Trash, Bot } from "lucide-react";
import { useEffect, useId, useRef, useState, type FormEvent } from "react";
import { toCommandError } from "../api/errors";
import type { Auth, Forward, Profile, ProfileId, SecretKind } from "../api/types";
import type { MessageKey } from "../i18n";
import { isActive } from "../state/AppStore";
import { useAppState, useI18n, useServices, useStore } from "../state/services";
import { cx } from "../util/cx";
import { Modal } from "./Modal";
import { Button, Field, IconButton, Segmented, ToggleRow } from "./ui";

const LOOPBACK = "127.0.0.1";

interface Problem {
  field: string;
  forwardIndex: number | null;
}

function blankForward(): Forward {
  return { id: "", label: "", bindHost: LOOPBACK, localPort: 0, remoteHost: LOOPBACK, remotePort: 0 };
}

function blankProfile(): Profile {
  return {
    id: "",
    name: "",
    host: "",
    port: 22,
    username: "",
    auth: { type: "password" },
    forwards: [blankForward()],
    autoReconnect: true,
    connectOnLaunch: false,
  };
}

/** A copy to edit: the server being edited, or one found in the SSH config, or an empty one. */
function startingPoint(profile: Profile | undefined): Profile {
  if (!profile) return blankProfile();
  const copy = structuredClone(profile);
  return copy.forwards.length > 0 ? copy : { ...copy, forwards: [blankForward()] };
}

const showPort = (port: number) => (port === 0 ? "" : String(port));
const readPort = (text: string) => Number(text.replace(/\D/g, "").slice(0, 5)) || 0;
const validPort = (port: number) => port >= 1 && port <= 65535;
const overlap = (a: string, b: string) => (a || LOOPBACK) === (b || LOOPBACK) || [a, b].some((h) => h === "0.0.0.0" || h === "::");

/** The first thing wrong with the form, by the same rules the core applies. */
function check(draft: Profile): Problem | null {
  const bad = (field: string, forwardIndex: number | null = null) => ({ field, forwardIndex });
  if (!draft.host.trim() || /\s/.test(draft.host.trim())) return bad("host");
  if (!validPort(draft.port)) return bad("port");
  if (!draft.username.trim()) return bad("username");
  if (draft.auth.type === "key" && !draft.auth.path.trim()) return bad("keyPath");
  if (draft.forwards.length === 0) return bad("forwards");
  for (const [i, forward] of draft.forwards.entries()) {
    if (!validPort(forward.localPort)) return bad("localPort", i);
    if (!validPort(forward.remotePort)) return bad("remotePort", i);
    const taken = draft.forwards
      .slice(0, i)
      .some((other) => other.localPort === forward.localPort && overlap(other.bindHost, forward.bindHost));
    if (taken) return bad("duplicatePort", i);
  }
  return null;
}

/** Whether saving `after` over `before` would reconnect a running server. */
function reconnects(before: Profile, after: Profile): boolean {
  const essentials = (p: Profile) => JSON.stringify({ ...p, name: "", connectOnLaunch: false });
  return essentials(before) !== essentials(after);
}

/**
 * The form for a server: new, or the one with `profileId`. `imported` (a server found in the SSH
 * config that still needs its ports) opens it as a new server with the fields filled in.
 */
export function ProfileDialog({
  profileId,
  imported,
  onClose,
}: {
  profileId: ProfileId | null;
  imported?: Profile;
  onClose: () => void;
}) {
  const i18n = useI18n();
  const { t } = i18n;
  const store = useStore();
  const { backend } = useServices();
  const original = useAppState((state) => state.profiles.find((p) => p.id === profileId));
  const running = useAppState((state) => isActive(profileId ? state.runtimes[profileId] : undefined));

  const [draft, setDraft] = useState<Profile>(() => startingPoint(original ?? imported));
  const [problem, setProblem] = useState<Problem | null>(null);
  const [advanced, setAdvanced] = useState(() => draft.forwards.some((f) => f.bindHost !== LOOPBACK));
  const [saved, setSaved] = useState<SecretKind[]>([]);
  const [busy, setBusy] = useState(false);
  const form = useRef<HTMLFormElement>(null);
  const shake = useAnimationControls();
  const nameId = useId();
  const hostId = useId();
  const portId = useId();
  const userId = useId();
  const keyId = useId();

  useEffect(() => {
    if (profileId) void store.savedSecrets(profileId).then(setSaved);
  }, [store, profileId]);

  const change = (patch: Partial<Profile>) => {
    setProblem(null);
    setDraft((d) => ({ ...d, ...patch }));
  };
  const changeForward = (index: number, patch: Partial<Forward>) => {
    setProblem(null);
    setDraft((d) => ({ ...d, forwards: d.forwards.map((f, i) => (i === index ? { ...f, ...patch } : f)) }));
  };
  /** The remote port follows the local one for as long as they match, which is the usual case. */
  const changeLocalPort = (index: number, localPort: number) => {
    const forward = draft.forwards[index];
    const follows = forward.remotePort === 0 || forward.remotePort === forward.localPort;
    changeForward(index, follows ? { localPort, remotePort: localPort } : { localPort });
  };

  const message = (field: string, forwardIndex: number | null = null): string | null =>
    problem && problem.field === field && problem.forwardIndex === forwardIndex
      ? t(`invalid.${field}` as MessageKey)
      : null;
  const rowProblem = (index: number): string | null =>
    problem?.forwardIndex === index ? t(`invalid.${problem.field}` as MessageKey) : null;

  const reject = (found: Problem) => {
    setProblem(found);
    void shake.start({ x: [0, -9, 8, -5, 3, 0], transition: { duration: 0.38 } });
    const target = found.forwardIndex === null ? found.field : `${found.field}-${found.forwardIndex}`;
    requestAnimationFrame(() => {
      const element = form.current?.querySelector<HTMLElement>(`[data-field="${target}"]`);
      element?.scrollIntoView({ block: "center", behavior: "smooth" });
      element?.focus({ preventScroll: true });
    });
  };

  const submit = async (connect: boolean) => {
    const found = check(draft);
    if (found) return reject(found);
    setBusy(true);
    try {
      const profile = await store.saveProfile(tidy(draft));
      if (connect) void store.connect(profile.id);
      onClose();
    } catch (raw) {
      const { error } = toCommandError(raw);
      if (error.code === "invalid") reject({ field: error.field, forwardIndex: error.forwardIndex });
    } finally {
      setBusy(false);
    }
  };

  const setAuth = (type: Auth["type"]) => {
    if (type === "key") change({ auth: { type, path: draft.auth.type === "key" ? draft.auth.path : "" } });
    else change({ auth: { type } });
  };

  const browse = async () => {
    try {
      const path = await backend.pickKeyFile();
      if (path) change({ auth: { type: "key", path } });
    } catch (raw) {
      store.reportError(raw);
    }
  };

  const secretKind: SecretKind | null =
    draft.auth.type === "password" ? "password" : draft.auth.type === "key" ? "passphrase" : null;
  const hasSavedSecret = secretKind !== null && saved.includes(secretKind);
  const restartNote = running && original && reconnects(original, tidy(draft));

  const title = t(original ? "profile.edit.title" : "profile.new.title");

  return (
    <Modal title={title} onClose={onClose} wide>
      <motion.form
        ref={form}
        className="modal__form"
        animate={shake}
        noValidate
        onSubmit={(event: FormEvent) => {
          event.preventDefault();
          void submit(false);
        }}
      >
        <header className="modal__head">
          <h2>{title}</h2>
        </header>

        {imported && <p className="notice modal__notice">{t("profile.imported")}</p>}

        <div className="modal__scroll">
          <section className="form-section">
            <h3>{t("profile.section.server")}</h3>
            <Field label={t("profile.name")} hint={t("profile.name.hint")} htmlFor={nameId}>
              <input
                id={nameId}
                className="input"
                value={draft.name}
                onChange={(e) => change({ name: e.target.value })}
                data-field="name"
                autoComplete="off"
              />
            </Field>
            <div className="form-row form-row--host">
              <Field label={t("profile.host")} error={message("host")} htmlFor={hostId}>
                <input
                  id={hostId}
                  className="input"
                  value={draft.host}
                  placeholder={t("profile.host.placeholder")}
                  onChange={(e) => change({ host: e.target.value })}
                  aria-invalid={message("host") !== null}
                  data-field="host"
                  data-autofocus={original || imported ? undefined : ""}
                  autoComplete="off"
                  spellCheck={false}
                />
              </Field>
              <Field label={t("profile.port")} error={message("port")} htmlFor={portId}>
                <input
                  id={portId}
                  className="input input--mono"
                  inputMode="numeric"
                  value={showPort(draft.port)}
                  onChange={(e) => change({ port: readPort(e.target.value) })}
                  aria-invalid={message("port") !== null}
                  data-field="port"
                />
              </Field>
            </div>
            <Field label={t("profile.username")} error={message("username")} htmlFor={userId}>
              <input
                id={userId}
                className="input"
                value={draft.username}
                onChange={(e) => change({ username: e.target.value })}
                aria-invalid={message("username") !== null}
                data-field="username"
                autoComplete="off"
                spellCheck={false}
              />
            </Field>
          </section>

          <section className="form-section">
            <h3>{t("profile.section.auth")}</h3>
            <Segmented<Auth["type"]>
              label={t("profile.section.auth")}
              value={draft.auth.type}
              onChange={setAuth}
              options={[
                { value: "password", label: t("profile.auth.password"), icon: <Lock size={14} /> },
                { value: "key", label: t("profile.auth.key"), icon: <KeyRound size={14} /> },
                { value: "agent", label: t("profile.auth.agent"), icon: <Bot size={14} /> },
              ]}
            />
            <p className="form-note">{t(`profile.auth.${draft.auth.type}.hint` as MessageKey)}</p>
            {draft.auth.type === "key" && (
              <Field label={t("profile.keyPath")} error={message("keyPath")} htmlFor={keyId}>
                <div className="input-group">
                  <input
                    id={keyId}
                    className="input input--mono"
                    value={draft.auth.path}
                    placeholder={t("profile.keyPath.placeholder")}
                    onChange={(e) => change({ auth: { type: "key", path: e.target.value } })}
                    aria-invalid={message("keyPath") !== null}
                    data-field="keyPath"
                    autoComplete="off"
                    spellCheck={false}
                  />
                  <Button icon={<FolderOpen size={15} />} onClick={() => void browse()}>
                    {t("profile.keyPath.browse")}
                  </Button>
                </div>
              </Field>
            )}
            {hasSavedSecret && profileId && secretKind && (
              <div className="notice">
                <span>{t(`profile.saved.${secretKind}` as MessageKey)}</span>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => void store.forgetSecrets(profileId).then(() => setSaved([]))}
                >
                  {t("common.forget")}
                </Button>
              </div>
            )}
          </section>

          <section className="form-section">
            <div className="form-section__head">
              <h3>{t("profile.section.ports")}</h3>
              <button type="button" className="link" aria-expanded={advanced} onClick={() => setAdvanced((a) => !a)}>
                {t("form.advanced")}
              </button>
            </div>
            {message("forwards") && (
              <p className="field__error" role="alert">
                {message("forwards")}
              </p>
            )}
            <div className="mapping-form">
              <div className={cx("mapping-form__titles", advanced && "is-advanced")}>
                <span>{t("form.listenOn")}</span>
                <span />
                <span>{t("form.forwardTo")}</span>
                <span />
              </div>
              {draft.forwards.map((forward, index) => (
                <div key={index} className={cx("mapping-form__row", rowProblem(index) && "has-problem")}>
                  <div className={cx("mapping-form__line", advanced && "is-advanced")}>
                    <div className={cx("mapping-form__side", advanced && "is-advanced")}>
                      {advanced && (
                        <>
                          <input
                            className="input input--mono"
                            aria-label={t("form.bindHost")}
                            title={t("form.bindHost.hint")}
                            value={forward.bindHost}
                            onChange={(e) => changeForward(index, { bindHost: e.target.value })}
                            aria-invalid={problem?.forwardIndex === index && problem.field === "bindHost"}
                            data-field={`bindHost-${index}`}
                            spellCheck={false}
                          />
                          <span aria-hidden="true">:</span>
                        </>
                      )}
                      <input
                        className="input input--mono"
                        inputMode="numeric"
                        aria-label={t("form.localPort")}
                        placeholder="8080"
                        value={showPort(forward.localPort)}
                        onChange={(e) => changeLocalPort(index, readPort(e.target.value))}
                        aria-invalid={
                          problem?.forwardIndex === index &&
                          (problem.field === "localPort" || problem.field === "duplicatePort")
                        }
                        data-field={`localPort-${index}`}
                        data-autofocus={imported && index === 0 && forward.localPort === 0 ? "" : undefined}
                      />
                    </div>
                    <span className="mapping-form__arrow" aria-hidden="true">
                      →
                    </span>
                    <div className="mapping-form__remote">
                      <input
                        className="input input--mono"
                        aria-label={t("form.remoteHost")}
                        title={t("form.remoteHost.hint")}
                        value={forward.remoteHost}
                        onChange={(e) => changeForward(index, { remoteHost: e.target.value })}
                        data-field={`remoteHost-${index}`}
                        spellCheck={false}
                      />
                      <span aria-hidden="true">:</span>
                      <input
                        className="input input--mono"
                        inputMode="numeric"
                        aria-label={t("form.remotePort")}
                        placeholder="80"
                        value={showPort(forward.remotePort)}
                        onChange={(e) => changeForward(index, { remotePort: readPort(e.target.value) })}
                        aria-invalid={problem?.forwardIndex === index && problem.field === "remotePort"}
                        data-field={`remotePort-${index}`}
                      />
                    </div>
                    <IconButton
                      label={t("form.remove")}
                      danger
                      disabled={draft.forwards.length === 1}
                      onClick={() => change({ forwards: draft.forwards.filter((_, i) => i !== index) })}
                    >
                      <Trash size={15} />
                    </IconButton>
                  </div>
                  <input
                    className="input input--label"
                    aria-label={t("form.label")}
                    placeholder={t("form.label.placeholder")}
                    value={forward.label}
                    onChange={(e) => changeForward(index, { label: e.target.value })}
                  />
                  {rowProblem(index) && (
                    <p className="field__error" role="alert">
                      {rowProblem(index)}
                    </p>
                  )}
                </div>
              ))}
              <Button
                size="sm"
                icon={<Plus size={14} />}
                onClick={() => change({ forwards: [...draft.forwards, blankForward()] })}
                data-field="forwards"
              >
                {t("form.add")}
              </Button>
            </div>
          </section>

          <section className="form-section">
            <h3>{t("profile.section.options")}</h3>
            <ToggleRow
              label={t("profile.autoReconnect")}
              hint={t("profile.autoReconnect.hint")}
              checked={draft.autoReconnect}
              onChange={(autoReconnect) => change({ autoReconnect })}
            />
            <ToggleRow
              label={t("profile.connectOnLaunch")}
              checked={draft.connectOnLaunch}
              onChange={(connectOnLaunch) => change({ connectOnLaunch })}
            />
          </section>
        </div>

        <footer className="modal__footer">
          {restartNote && <p className="modal__footnote">{t("profile.restartNote")}</p>}
          <Button onClick={onClose}>{t("common.cancel")}</Button>
          <Button type="submit" variant={running ? "primary" : "secondary"} disabled={busy}>
            {t("common.save")}
          </Button>
          {!running && (
            <Button variant="primary" disabled={busy} onClick={() => void submit(true)}>
              {t("profile.saveAndConnect")}
            </Button>
          )}
        </footer>
      </motion.form>
    </Modal>
  );
}

/** What is sent to the core: the same, with the stray spaces gone. */
function tidy(draft: Profile): Profile {
  return {
    ...draft,
    name: draft.name.trim(),
    host: draft.host.trim(),
    username: draft.username.trim(),
    auth: draft.auth.type === "key" ? { type: "key", path: draft.auth.path.trim() } : draft.auth,
    forwards: draft.forwards.map((f) => ({
      ...f,
      label: f.label.trim(),
      bindHost: f.bindHost.trim(),
      remoteHost: f.remoteHost.trim(),
    })),
  };
}
