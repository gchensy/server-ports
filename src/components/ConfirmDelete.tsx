import { useState } from "react";
import { useI18n, useStore } from "../state/services";
import { Modal } from "./Modal";
import { Button } from "./ui";

export function ConfirmDelete({ profileId, onClose }: { profileId: string; onClose: () => void }) {
  const { t } = useI18n();
  const store = useStore();
  // Read once: the name must not vanish from the dialog while it fades out.
  const [name] = useState(() => store.getState().profiles.find((p) => p.id === profileId)?.name ?? "");
  const title = t("delete.title", { name });
  return (
    <Modal title={title} onClose={onClose}>
      <div className="modal__form">
        <div className="modal__body">
          <h2>{title}</h2>
          <p>{t("delete.body")}</p>
        </div>
        <footer className="modal__footer">
          <Button data-autofocus onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button
            variant="danger"
            onClick={() => {
              onClose();
              void store.deleteProfile(profileId);
            }}
          >
            {t("delete.confirm")}
          </Button>
        </footer>
      </div>
    </Modal>
  );
}
