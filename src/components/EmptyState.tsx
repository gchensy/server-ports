import { FileTerminal, Plus } from "lucide-react";
import { motion } from "motion/react";
import { useI18n, useSshHosts } from "../state/services";
import { gentle } from "./motion";
import { Button } from "./ui";

/** The first thing a new user sees: what the app is for, and where to start. */
export function EmptyState({ onAdd, onImport }: { onAdd: () => void; onImport: () => void }) {
  const i18n = useI18n();
  const { t } = i18n;
  // A quiet look at the SSH config: it is only worth a word if it holds servers.
  const found = useSshHosts()?.length ?? 0;

  return (
    <motion.div
      className="empty"
      initial={{ opacity: 0, y: 14 }}
      animate={{ opacity: 1, y: 0 }}
      transition={gentle}
    >
      <svg className="empty__art" viewBox="0 0 360 150" aria-hidden="true">
        <g className="art-computer">
          <rect x="22" y="30" width="92" height="62" rx="9" className="art-body" />
          <rect x="32" y="40" width="72" height="42" rx="5" className="art-screen" />
          <path d="M54 110h28M68 92v18" className="art-stroke" />
        </g>
        <g className="art-server">
          {[22, 54, 86].map((y, i) => (
            <g key={y}>
              <rect x="250" y={y} width="90" height="28" rx="8" className="art-body" />
              <circle cx="267" cy={y + 14} r="3.2" className={`art-led art-led--${i}`} />
              <path d={`M282 ${y + 14}h42`} className="art-stroke" />
            </g>
          ))}
        </g>
        <path d="M118 62 C168 62 196 70 246 70" className="art-route" />
        {/* On the route, standing still: a picture that moves for good keeps the window repainting. */}
        {[
          [152, 63],
          [182, 66],
          [212, 69],
        ].map(([x, y]) => (
          <circle key={x} cx={x} cy={y} r="4" className="art-packet" />
        ))}
      </svg>
      <h1>{t("empty.title")}</h1>
      <p>{t("empty.body")}</p>
      <Button variant="primary" icon={<Plus size={16} />} onClick={onAdd}>
        {t("empty.cta")}
      </Button>
      {found > 0 && (
        <motion.div
          className="empty__found"
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          transition={gentle}
        >
          <FileTerminal size={18} aria-hidden="true" />
          <span>{i18n.plural("empty.found", found)}</span>
          <Button size="sm" onClick={onImport}>
            {t("empty.import")}
          </Button>
        </motion.div>
      )}
    </motion.div>
  );
}
