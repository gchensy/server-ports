import { Eye, EyeOff, KeyRound } from "lucide-react";
import { useId, useState, type FormEvent } from "react";
import type { SecretEvent } from "../api/types";
import { useI18n, useStore } from "../state/services";
import { Modal } from "./Modal";
import { Button, IconButton } from "./ui";

/** "Enter the password / key passphrase", with the choice to keep it in the system keychain. */
export function SecretDialog({ prompt }: { prompt: SecretEvent }) {
  const { t } = useI18n();
  const store = useStore();
  const [value, setValue] = useState("");
  const [shown, setShown] = useState(false);
  const [remember, setRemember] = useState(false);
  const inputId = useId();
  const rememberId = useId();

  const cancel = () => store.answerSecret(prompt.requestId, null, false);
  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (value) store.answerSecret(prompt.requestId, value, remember);
  };
  const title = t(`secret.${prompt.kind}.title`, { name: prompt.profileName });

  return (
    <Modal title={title} onClose={cancel}>
      <form className="modal__form" onSubmit={submit}>
        <div className="modal__body">
          <div className="modal__icon">
            <KeyRound size={22} />
          </div>
          <h2>{title}</h2>
          {prompt.retry && (
            <p className="notice notice--error" role="alert">
              {t("secret.retry")}
            </p>
          )}
          <label htmlFor={inputId} className="sr-only">
            {t(`secret.${prompt.kind}.label`)}
          </label>
          <div className="input-group">
            <input
              id={inputId}
              data-autofocus
              className="input"
              type={shown ? "text" : "password"}
              placeholder={t(`secret.${prompt.kind}.label`)}
              value={value}
              onChange={(event) => setValue(event.target.value)}
              autoComplete="off"
              spellCheck={false}
            />
            <IconButton label={t(shown ? "secret.hide" : "secret.show")} onClick={() => setShown((s) => !s)}>
              {shown ? <EyeOff size={16} /> : <Eye size={16} />}
            </IconButton>
          </div>
          <label className="check" htmlFor={rememberId}>
            <input id={rememberId} type="checkbox" checked={remember} onChange={(e) => setRemember(e.target.checked)} />
            <span>{t("secret.remember")}</span>
          </label>
        </div>
        <footer className="modal__footer">
          <Button onClick={cancel}>{t("common.cancel")}</Button>
          <Button type="submit" variant="primary" disabled={!value}>
            {t("secret.submit")}
          </Button>
        </footer>
      </form>
    </Modal>
  );
}
