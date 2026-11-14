import { invoke, isTauri } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

export interface LogsDir {
  path: string | null | undefined;
  choose: () => void;
}

export function useLogsDir(): LogsDir {
  const [path, setPath] = useState<string | null | undefined>(undefined);

  useEffect(() => {
    if (!isTauri()) return;
    let current = true;
    invoke<string | null>("logs_dir")
      .then((saved) => {
        if (current) setPath(saved);
      })
      .catch(report);
    return () => {
      current = false
    };
  }, []);

  const choose = () => {
    invoke<string | null>("choose_logs_dir")
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
export function useCombatLogs(folder: string | null | undefined): CombatLogs {
  const [names, setNames] = useState<string[]>([]);
  const [asked, setAsked] = useState(0);

  useEffect(() => {
    if (!isTauri() || !folder) return;
    let current = true;
    invoke<string[]>("list_logs")
      .then((found) => {
        if (current) setNames(found);
      })
      .catch((error: unknown) => console.error("[corvid] the combat logs coulndn't be listed", error))
    return () => {
      current = false;
    };
  }, [folder, asked])
  
  return { names, refresh: () => setAsked((n) => n + 1) };
}

function report(error: unknown) {
  console.error("[corvid] the combat log folder couldn't be loaded or saved", error);
}