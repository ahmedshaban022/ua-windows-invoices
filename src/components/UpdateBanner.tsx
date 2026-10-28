import { useCallback, useEffect, useState } from "react";
import { Download, X } from "lucide-react";

import { Button } from "./ui";
import * as api from "../lib/api";
import type { UpdateStatus } from "../lib/types";
import { toast } from "../state/toast";

/**
 * The update notification (PRD FR-11).
 *
 * The check is silent and the update is never installed without a click. The
 * user can postpone it, in which case the same version is not offered again.
 */
export function UpdateBanner() {
  const [status, setStatus] = useState<UpdateStatus | null>(null);
  const [visible, setVisible] = useState(false);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<{ downloaded: number; total: number | null } | null>(
    null,
  );

  const check = useCallback(async () => {
    try {
      const deferred = await api.deferredUpdate();
      const result = await api.checkForUpdates();
      if (!result.updateAvailable) return;
      if (deferred && deferred === result.latestVersion) return;
      setStatus(result);
      setVisible(true);
    } catch {
      // Offline or an unreachable feed is not an error the user must see.
    }
  }, []);

  useEffect(() => {
    // Give the application a moment to finish starting before checking.
    const handle = window.setTimeout(() => void check(), 4000);
    return () => window.clearTimeout(handle);
  }, [check]);

  useEffect(() => {
    const unlisten = async () => {
      const { listen } = await import("@tauri-apps/api/event");
      const stop = await listen<{ downloaded: number; total: number | null }>(
        "update-progress",
        (event) => setProgress(event.payload),
      );
      return stop;
    };
    let dispose: (() => void) | undefined;
    void unlisten().then((stop) => {
      dispose = stop;
    });
    return () => dispose?.();
  }, []);

  const install = async () => {
    if (!status?.latestVersion) return;
    setBusy(true);
    setProgress({ downloaded: 0, total: null });
    try {
      await api.installUpdate();
      toast("جاري تثبيت التحديث…", "success");
    } catch (error) {
      toast(api.errorMessage(error), "error");
      setBusy(false);
      setProgress(null);
    }
  };

  const later = async () => {
    if (status?.latestVersion) {
      await api.deferUpdate(status.latestVersion).catch(() => undefined);
    }
    setVisible(false);
  };

  if (!visible || !status) return null;

  return (
    <div className="update-banner" role="status">
      <div className="update-banner__body">
        <div className="update-banner__title">يتوفر تحديث جديد</div>
        <div className="update-banner__text">
          الإصدار {status.latestVersion} — الإصدار الحالي {status.currentVersion}.
        </div>
        {status.notes ? <div className="update-banner__notes">{status.notes}</div> : null}
        {progress ? (
          <div className="update-banner__progress">
            <div
              className="update-banner__bar"
              style={{
                width: progress.total
                  ? `${Math.min(100, Math.round((progress.downloaded / progress.total) * 100))}%`
                  : "30%",
              }}
            />
          </div>
        ) : null}
      </div>
      <div className="update-banner__actions">
        <Button variant="primary" size="sm" onClick={install} loading={busy}>
          <Download size={15} aria-hidden="true" /> تحديث الآن
        </Button>
        <Button variant="soft" size="sm" onClick={later}>
          لاحقًا
        </Button>
        <Button variant="ghost" size="sm" onClick={() => setVisible(false)} aria-label="إخفاء">
          <X size={15} aria-hidden="true" />
        </Button>
      </div>
    </div>
  );
}
