import { invoke, isTauri } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

export interface WowDir {
  path: string | null | undefined;
  choose: () => void;
}

export function useWowDir(): WowDir {
  const [path, setPath] = useState<string | null | undefined>(undefined);

  useEffect(() => {
    if (!isTauri()) return;
    let current = true;
    invoke<string | null>("wow_dir")
      .then((saved) => {
        if (current) setPath(saved);
      })
      .catch(report);
    return () => {
      current = false
    };
  }, []);

  const choose = () => {
    invoke<string | null>("choose_wow_dir")
      .then((chosen) => {
        if (chosen !== null) setPath(chosen);
      })
      .catch(report);
  };

  return { path, choose };
}

export interface CombatLogs {
  names: string[];
  refresh: () => void;
}

/** The combat logs in `folder`, newest first, for the replay picker. */
export function useCombatLogs(wowDir: string | null | undefined): CombatLogs {
  const [names, setNames] = useState<string[]>([]);
  const [asked, setAsked] = useState(0);

  useEffect(() => {
    if (!isTauri() || !wowDir) return;
    let current = true;
    invoke<string[]>("list_logs")
      .then((found) => {
        if (current) setNames(found);
      })
      .catch((error: unknown) => console.error("[corvid] the combat logs couldn't be listed", error))
    return () => {
      current = false;
    };
  }, [wowDir, asked])
  
  return { names, refresh: () => setAsked((n) => n + 1) };
}

function report(error: unknown) {
  console.error("[corvid] the combat log folder couldn't be loaded or saved", error);
}