import { useTranslation } from "react-i18next";
import { usePythonEnv } from "@/contexts/PythonEnvContext";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { AlertCircle, CheckCircle2, Loader2, Terminal } from "lucide-react";

interface PythonEnvDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onComplete?: () => void;
}

export function PythonEnvDialog({
  open,
  onOpenChange,
  onComplete,
}: PythonEnvDialogProps) {
  const { t } = useTranslation();
  const {
    status,
    isInstalling,
    installProgress,
    installMessage,
    error,
    ensureReady,
  } = usePythonEnv();

  const handleInstall = async () => {
    await ensureReady();
    if (status === "ready") {
      onComplete?.();
    }
  };

  const handleClose = () => {
    if (!isInstalling) {
      onOpenChange(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Terminal className="h-5 w-5" />
            {t("pythonEnv.title", "Python Environment Setup")}
          </DialogTitle>
          <DialogDescription>
            {t(
              "pythonEnv.description",
              "Qwen3-ASR requires a Python environment with the qwen-asr package. This will be installed automatically."
            )}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {status === "ready" && (
            <div className="flex items-center gap-3 rounded-lg border border-green-200 bg-green-50 p-4 dark:border-green-800 dark:bg-green-950/30">
              <CheckCircle2 className="h-5 w-5 text-green-600 dark:text-green-400" />
              <div className="text-sm">
                <p className="font-medium text-green-800 dark:text-green-200">
                  {t("pythonEnv.ready", "Environment Ready")}
                </p>
                <p className="text-green-600 dark:text-green-400">
                  {t(
                    "pythonEnv.readyDesc",
                    "qwen-asr is installed and ready to use."
                  )}
                </p>
              </div>
            </div>
          )}

          {(status === "notReady" || status === "failed") && !isInstalling && (
            <div className="space-y-3">
              <div className="rounded-lg border p-4">
                <h4 className="text-sm font-medium">
                  {t("pythonEnv.willInstall", "What will be installed:")}
                </h4>
                <ul className="mt-2 space-y-1 text-sm text-muted-foreground">
                  <li>• {t("pythonEnv.venv", "Python virtual environment")}</li>
                  <li>• {t("pythonEnv.qwenAsr", "qwen-asr package")}</li>
                  <li>
                    •{" "}
                    {t(
                      "pythonEnv.pytorch",
                      "PyTorch (CPU by default, GPU option available after)"
                    )}
                  </li>
                </ul>
              </div>

              {error && (
                <div className="flex items-start gap-3 rounded-lg border border-red-200 bg-red-50 p-4 dark:border-red-800 dark:bg-red-950/30">
                  <AlertCircle className="h-5 w-5 text-red-600 dark:text-red-400 mt-0.5" />
                  <div className="text-sm">
                    <p className="font-medium text-red-800 dark:text-red-200">
                      {t("pythonEnv.error", "Installation Failed")}
                    </p>
                    <p className="mt-1 text-red-600 dark:text-red-400 break-all">
                      {error}
                    </p>
                  </div>
                </div>
              )}
            </div>
          )}

          {isInstalling && (
            <div className="space-y-4">
              <div className="flex items-center gap-3">
                <Loader2 className="h-5 w-5 animate-spin text-primary" />
                <span className="text-sm font-medium">{installMessage}</span>
              </div>
              <Progress value={installProgress} className="w-full" />
              <p className="text-xs text-muted-foreground text-center">
                {t(
                  "pythonEnv.pleaseWait",
                  "This may take a few minutes on first install..."
                )}
              </p>
            </div>
          )}
        </div>

        <DialogFooter>
          {status === "ready" && (
            <Button onClick={handleClose}>
              {t("common.done", "Done")}
            </Button>
          )}

          {(status === "notReady" || status === "failed") && !isInstalling && (
            <>
              <Button variant="outline" onClick={handleClose}>
                {t("common.cancel", "Cancel")}
              </Button>
              <Button onClick={handleInstall}>
                {status === "failed"
                  ? t("pythonEnv.retry", "Retry Installation")
                  : t("pythonEnv.install", "Install Now")}
              </Button>
            </>
          )}

          {isInstalling && (
            <Button variant="outline" disabled>
              {t("pythonEnv.installing", "Installing...")}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
