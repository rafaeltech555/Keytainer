import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Setup } from "./routes/Setup";
import { Unlock } from "./routes/Unlock";
import { List } from "./routes/List";
import { ItemDetail } from "./routes/ItemDetail";
import { Settings } from "./routes/Settings";
import { ipc } from "./lib/ipc";
import { I18nProvider, useT } from "./lib/i18n";
import "./App.css";

type Screen =
  | { kind: "loading" }
  | { kind: "setup" }
  | { kind: "unlock"; reason?: "idle" | "manual" }
  | { kind: "list" }
  | { kind: "detail"; itemId: string | "new" }
  | { kind: "settings" };

function AppInner() {
  const t = useT();
  const [screen, setScreen] = useState<Screen>({ kind: "loading" });
  const [refreshKey, setRefreshKey] = useState(0);
  const unlocked = useRef(false);

  useEffect(() => {
    void boot();

    const unlistenPromise = listen<string>("vault-locked", (event) => {
      unlocked.current = false;
      setScreen({
        kind: "unlock",
        reason: event.payload === "idle" ? "idle" : "manual",
      });
    });

    return () => {
      void unlistenPromise.then((fn) => fn());
    };
  }, []);

  // Keep the idle auto-lock timer alive on raw user interaction, not just on
  // IPC calls — otherwise a user typing in a form for a while gets locked out
  // mid-edit. Throttled to at most one ping every 20s. Only while unlocked.
  useEffect(() => {
    let last = 0;
    const onActivity = () => {
      if (!unlocked.current) return;
      const now = Date.now();
      if (now - last < 20_000) return;
      last = now;
      void ipc.pingActivity().catch(() => {});
    };
    const events: (keyof WindowEventMap)[] = ["keydown", "pointerdown", "pointermove"];
    events.forEach((e) => window.addEventListener(e, onActivity, { passive: true }));
    return () => events.forEach((e) => window.removeEventListener(e, onActivity));
  }, []);

  async function boot() {
    const exists = await ipc.vaultExists();
    if (!exists) {
      setScreen({ kind: "setup" });
      return;
    }
    const isUnlocked = await ipc.isUnlocked();
    unlocked.current = isUnlocked;
    setScreen({ kind: isUnlocked ? "list" : "unlock" });
  }

  function enterUnlocked(next: Screen) {
    unlocked.current = true;
    setScreen(next);
  }

  function bumpList() {
    setRefreshKey((k) => k + 1);
  }

  switch (screen.kind) {
    case "loading":
      return <div className="screen centered"><p>{t("loading")}</p></div>;

    case "setup":
      return <Setup onCreated={() => enterUnlocked({ kind: "list" })} />;

    case "unlock":
      return (
        <Unlock
          reason={screen.reason}
          onUnlocked={() => enterUnlocked({ kind: "list" })}
        />
      );

    case "list":
      return (
        <List
          refreshKey={refreshKey}
          onSelect={(id) => setScreen({ kind: "detail", itemId: id })}
          onLock={() => {
            unlocked.current = false;
            setScreen({ kind: "unlock", reason: "manual" });
          }}
          onSettings={() => setScreen({ kind: "settings" })}
        />
      );

    case "detail":
      return (
        <ItemDetail
          itemId={screen.itemId}
          onClose={() => setScreen({ kind: "list" })}
          onSaved={() => {
            bumpList();
            setScreen({ kind: "list" });
          }}
          onDeleted={() => {
            bumpList();
            setScreen({ kind: "list" });
          }}
        />
      );

    case "settings":
      return (
        <Settings
          onClose={() => {
            bumpList();
            setScreen({ kind: "list" });
          }}
        />
      );
  }
}

export default function App() {
  return (
    <I18nProvider>
      <AppInner />
    </I18nProvider>
  );
}
