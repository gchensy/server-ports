import { Monitor, Moon, Power, Sun, X } from "lucide-react";
import { useEffect, useState } from "react";
import type { Language } from "../i18n";
import type { Theme } from "../state/Appearance";
import { useAppearance, useAppState, useI18n, useServices } from "../state/services";
import { LanguagePicker } from "./LanguagePicker";
import { Modal } from "./Modal";
import { Button, IconButton, Segmented, ToggleRow } from "./ui";

export function SettingsDialog({ onClose }: { onClose: () => void }) {
  const { backend, store } = useServices();
  const i18n = useI18n();
  const { t } = i18n;
  const appearance = useAppearance();
  const settings = useAppState((state) => state.settings);
  const [autostart, setAutostart] = useState(false);

  useEffect(() => {
    backend.autostartEnabled().then(setAutostart, () => setAutostart(false));
  }, [backend]);

  const toggleAutostart = async (enabled: boolean) => {
    setAutostart(enabled);
    try {
      await backend.setAutostart(enabled);
    } catch (raw) {
      setAutostart(!enabled);
      store.reportError(raw);
    }
  };

  const chooseLanguage = (language: Language) => {
    i18n.setLanguage(language);
    void store.sendTrayLabels();
  };

  return (
    <Modal title={t("settings.title")} onClose={onClose}>
      <div className="modal__form">
        <header className="modal__head modal__head--row">
          <h2>{t("settings.title")}</h2>
          <IconButton label={t("common.close")} onClick={onClose}>
            <X size={16} />
          </IconButton>
        </header>
        <div className="modal__scroll">
          <section className="form-section">
            <h3>{t("settings.language")}</h3>
            <LanguagePicker
              label={t("settings.language")}
              systemLabel={t("settings.language.system")}
              value={i18n.language}
              onChange={chooseLanguage}
            />
          </section>
          <section className="form-section">
            <h3>{t("settings.theme")}</h3>
            <Segmented<Theme>
              label={t("settings.theme")}
              value={appearance.theme}
              onChange={(theme) => appearance.setTheme(theme)}
              options={[
                { value: "system", label: t("settings.theme.system"), icon: <Monitor size={14} /> },
                { value: "light", label: t("settings.theme.light"), icon: <Sun size={14} /> },
                { value: "dark", label: t("settings.theme.dark"), icon: <Moon size={14} /> },
              ]}
            />
          </section>
          <section className="form-section">
            <ToggleRow
              label={t("settings.closeToTray")}
              hint={t("settings.closeToTray.hint")}
              checked={settings.closeToTray}
              onChange={(closeToTray) => void store.setSettings({ ...settings, closeToTray })}
            />
            <ToggleRow
              label={t("settings.autostart")}
              hint={t("settings.autostart.hint")}
              checked={autostart}
              onChange={(enabled) => void toggleAutostart(enabled)}
            />
          </section>
        </div>
        <footer className="modal__footer modal__footer--split">
          <Button variant="danger" icon={<Power size={15} />} onClick={() => void backend.quit()}>
            {t("settings.quit")}
          </Button>
        </footer>
      </div>
    </Modal>
  );
}
