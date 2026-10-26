import { AnimatePresence, motion } from "motion/react";
import { useAppState, useStore } from "../state/services";
import { spring } from "./motion";

export function Toasts() {
  const store = useStore();
  const toasts = useAppState((state) => state.toasts);
  return (
    <div className="toasts" aria-live="polite">
      <AnimatePresence initial={false}>
        {toasts.map((toast) => (
          <motion.button
            key={toast.id}
            type="button"
            layout
            className="toast"
            initial={{ opacity: 0, y: 18, scale: 0.96 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, scale: 0.96, transition: { duration: 0.14 } }}
            transition={spring}
            onClick={() => store.dismissToast(toast.id)}
          >
            {toast.text}
          </motion.button>
        ))}
      </AnimatePresence>
    </div>
  );
}
