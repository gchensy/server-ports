import { ShieldAlert } from "lucide-react";
import type { HostKeyEvent } from "../api/types";
import { useI18n, useStore } from "../state/services";
import { Modal } from "./Modal";
import { Button } from "./ui";

/** "This server showed a key I have not seen. Trust it?" */
export function HostKeyDialog({ prompt }: { prompt: HostKeyEvent }) {
  const { t } = useI18n();
  const store = useStore();
  const reject = () => store.answerHostKey(prompt.requestId, false);
  return (
    <Modal title={t("hostKey.title")} onClose={reject}>
      <div className="modal__form">
        <div className="modal__body">
          <div className="modal__icon modal__icon--warn">
            <ShieldAlert size={22} />
          </div>
          <h2>{t("hostKey.title")}</h2>
          <p>{t("hostKey.body", { name: prompt.profileName, host: prompt.host, port: prompt.port })}</p>
          <dl className="facts">
            <dt>{t("hostKey.keyType")}</dt>
            <dd>
              <code>{prompt.keyType}</code>
            </dd>
            <dt>{t("hostKey.fingerprint")}</dt>
            <dd>
              <code className="facts__fingerprint">{prompt.fingerprint}</code>
            </dd>
          </dl>
          {prompt.otherKnown && <p className="notice notice--warn">{t("hostKey.otherKnown")}</p>}
        </div>
        <footer className="modal__footer">
          <Button data-autofocus onClick={reject}>
            {t("hostKey.reject")}
          </Button>
          <Button variant="primary" onClick={() => store.answerHostKey(prompt.requestId, true)}>
            {t("hostKey.trust")}
          </Button>
        </footer>
      </div>
    </Modal>
  );
}
