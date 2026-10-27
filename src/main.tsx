import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { createBackend } from "./api";
import { Services, ServicesProvider } from "./state/services";
import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/components.css";
import "./styles/app.css";

async function main() {
  const services = new Services(await createBackend());
  document.documentElement.lang = services.i18n.locale;
  createRoot(document.getElementById("root") as HTMLElement).render(
    <StrictMode>
      <ServicesProvider services={services}>
        <App />
      </ServicesProvider>
    </StrictMode>,
  );
  // The tray has its words before the window may go (a start in the tray closes it again once it
  // is ready), and the window is ready once the store has started.
  await services.store.sendTrayLabels();
  await services.store.start();
}

void main();
