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

function report(error: unknown) {
  console.error("[corvid] the combat log folder couldn't be loaded or saved", error);
}