import { AnimatePresence, MotionConfig } from "motion/react";
import { useEffect, useState } from "react";
import type { Profile, ProfileId } from "./api/types";
import { ConfirmDelete } from "./components/ConfirmDelete";
import { Detail } from "./components/Detail";
import { EmptyState } from "./components/EmptyState";
import { HostKeyDialog } from "./components/HostKeyDialog";
import { ProfileDialog } from "./components/ProfileDialog";
import { SecretDialog } from "./components/SecretDialog";
import { SettingsDialog } from "./components/SettingsDialog";
import { Sidebar } from "./components/Sidebar";
import { Toasts } from "./components/Toasts";
import { Button } from "./components/ui";
import { useAppState, useI18n, useServices } from "./state/services";

type Dialog =
  /** `from`: a new server that starts out on the list of the SSH config instead of the form. */
  | { kind: "profile"; id: ProfileId | null; from?: "ssh" }
  | { kind: "settings" }
  | { kind: "delete"; id: ProfileId };

export default function App() {
  const { backend, store } = useServices();
  const { t } = useI18n();
  const loaded = useAppState((state) => state.loaded);
  const loadError = useAppState((state) => state.loadError);
  const empty = useAppState((state) => state.profiles.length === 0);
  const prompt = useAppState((state) => state.prompts[0]);
  const [dialog, setDialog] = useState<Dialog | null>(null);
  // Servers picked from the SSH config that the form has to complete, one after the other.
  const [unsaved, setUnsaved] = useState<Profile[]>([]);
  const close = () => setDialog(null);
  const imported = dialog === null ? unsaved[0] : undefined;

  // The window is created hidden; it may appear once there is something to look at. A hidden
  // page paints no frames, so waiting for one would wait forever.
  useEffect(() => {
    if (loaded) void backend.uiReady();
  }, [loaded, backend]);

  if (loadError) {
    return (
      <div className="fatal" role="alert">
        <h1>{t("error.other.title")}</h1>
        <code>{loadError}</code>
        <Button onClick={() => void store.start()}>↻</Button>
      </div>
    );
  }
  if (!loaded) return null;

  return (
    <MotionConfig reducedMotion="user">
      <div className="app">
        <Sidebar onNew={() => setDialog({ kind: "profile", id: null })} onSettings={() => setDialog({ kind: "settings" })} />
        <main className="main">
          {empty ? (
            <EmptyState
              onAdd={() => setDialog({ kind: "profile", id: null })}
              onImport={() => setDialog({ kind: "profile", id: null, from: "ssh" })}
            />
          ) : (
            <Detail
              onEdit={(id) => setDialog({ kind: "profile", id })}
              onDelete={(id) => setDialog({ kind: "delete", id })}
            />
          )}
        </main>
      </div>
      <AnimatePresence>
        {dialog?.kind === "profile" && (
          <ProfileDialog
            key="profile"
            profileId={dialog.id}
            from={dialog.from}
            onClose={close}
            onFinishImport={(rest) => {
              setUnsaved(rest);
              close();
            }}
          />
        )}
        {imported && (
          <ProfileDialog
            key={`imported-${imported.name}`}
            profileId={null}
            imported={imported}
            onClose={() => setUnsaved((rest) => rest.slice(1))}
          />
        )}
        {dialog?.kind === "settings" && <SettingsDialog key="settings" onClose={close} />}
        {dialog?.kind === "delete" && <ConfirmDelete key="delete" profileId={dialog.id} onClose={close} />}
        {prompt?.type === "hostKey" && <HostKeyDialog key={prompt.requestId} prompt={prompt} />}
        {prompt?.type === "secret" && <SecretDialog key={prompt.requestId} prompt={prompt} />}
      </AnimatePresence>
      <Toasts />
    </MotionConfig>
  );
}
