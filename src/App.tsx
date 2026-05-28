import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Setup } from "./routes/Setup";
import { Unlock } from "./routes/Unlock";
import { List } from "./routes/List";
import { ItemDetail } from "./routes/ItemDetail";
import { Settings } from "./routes/Settings";
import { ipc } from "./lib/ipc";
import "./App.css";

type Screen =
  | { kind: "loading" }
  | { kind: "setup" }
  | { kind: "unlock"; reason?: "idle" | "manual" }
  | { kind: "list" }
  | { kind: "detail"; itemId: string | "new" }
  | { kind: "settings" };

export default function App() {
  const [screen, setScreen] = useState<Screen>({ kind: "loading" });
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    void boot();

    const unlistenPromise = listen<string>("vault-locked", (event) => {
      setScreen({
        kind: "unlock",
        reason: event.payload === "idle" ? "idle" : "manual",
      });
    });

    return () => {
      void unlistenPromise.then((fn) => fn());
    };
  }, []);

  async function boot() {
    const exists = await ipc.vaultExists();
    if (!exists) {
      setScreen({ kind: "setup" });
      return;
    }
    const unlocked = await ipc.isUnlocked();
    setScreen({ kind: unlocked ? "list" : "unlock" });
  }

  function bumpList() {
    setRefreshKey((k) => k + 1);
  }

  switch (screen.kind) {
    case "loading":
      return <div className="screen centered"><p>載入中…</p></div>;

    case "setup":
      return <Setup onCreated={() => setScreen({ kind: "list" })} />;

    case "unlock":
      return (
        <Unlock
          reason={screen.reason}
          onUnlocked={() => setScreen({ kind: "list" })}
        />
      );

    case "list":
      return (
        <List
          refreshKey={refreshKey}
          onSelect={(id) => setScreen({ kind: "detail", itemId: id })}
          onLock={() => setScreen({ kind: "unlock", reason: "manual" })}
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
