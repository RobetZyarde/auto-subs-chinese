import * as React from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type PythonEnvStatus = "ready" | "notReady" | "installing" | "failed";

export interface PythonInstallProgress {
  stage: string;
  percent: number;
  message: string;
}

interface PythonEnvContextValue {
  /** Current status of the Python environment. */
  status: PythonEnvStatus;
  /** Whether the environment is currently being set up. */
  isInstalling: boolean;
  /** Installation progress (0-100). */
  installProgress: number;
  /** Current installation stage message. */
  installMessage: string;
  /** Error message if installation failed. */
  error: string | null;
  /** Trigger Python environment setup. */
  ensureReady: () => Promise<void>;
  /** Install CUDA PyTorch for GPU acceleration. */
  installCuda: () => Promise<void>;
  /** Re-check environment status. */
  refresh: () => Promise<void>;
}

const PythonEnvContext = React.createContext<PythonEnvContextValue | null>(null);

export function PythonEnvProvider({ children }: { children: React.ReactNode }) {
  const [status, setStatus] = React.useState<PythonEnvStatus>("notReady");
  const [isInstalling, setIsInstalling] = React.useState(false);
  const [installProgress, setInstallProgress] = React.useState(0);
  const [installMessage, setInstallMessage] = React.useState("");
  const [error, setError] = React.useState<string | null>(null);

  // Check status on mount
  const refresh = React.useCallback(async () => {
    try {
      const result = await invoke<PythonEnvStatus>("check_python_env");
      setStatus(result);
    } catch (err) {
      console.error("[PythonEnv] failed to check status:", err);
      setStatus("notReady");
    }
  }, []);

  React.useEffect(() => {
    refresh();
  }, [refresh]);

  // Listen for progress events
  React.useEffect(() => {
    const unlisten = listen<PythonInstallProgress>(
      "python-install-progress",
      (event) => {
        const { stage, percent, message } = event.payload;
        setInstallProgress(percent);
        setInstallMessage(message);

        if (stage === "done") {
          setIsInstalling(false);
          setStatus("ready");
          setError(null);
        } else if (stage === "failed") {
          setIsInstalling(false);
          setStatus("failed");
          setError(message);
        }
      }
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const ensureReady = React.useCallback(async () => {
    if (isInstalling) return;

    setIsInstalling(true);
    setInstallProgress(0);
    setInstallMessage("Starting installation...");
    setError(null);
    setStatus("installing");

    try {
      await invoke("ensure_python_env");
      setStatus("ready");
    } catch (err) {
      const errorMsg = String(err);
      console.error("[PythonEnv] installation failed:", errorMsg);
      setError(errorMsg);
      setStatus("failed");
    } finally {
      setIsInstalling(false);
    }
  }, [isInstalling]);

  const installCuda = React.useCallback(async () => {
    if (isInstalling) return;

    setIsInstalling(true);
    setInstallProgress(0);
    setInstallMessage("Installing CUDA PyTorch...");
    setError(null);

    try {
      await invoke("install_cuda_pytorch");
    } catch (err) {
      const errorMsg = String(err);
      console.error("[PythonEnv] CUDA installation failed:", errorMsg);
      setError(errorMsg);
    } finally {
      setIsInstalling(false);
    }
  }, [isInstalling]);

  const value: PythonEnvContextValue = {
    status,
    isInstalling,
    installProgress,
    installMessage,
    error,
    ensureReady,
    installCuda,
    refresh,
  };

  return (
    <PythonEnvContext.Provider value={value}>
      {children}
    </PythonEnvContext.Provider>
  );
}

export function usePythonEnv() {
  const context = React.useContext(PythonEnvContext);
  if (!context) {
    throw new Error("usePythonEnv must be used within a PythonEnvProvider");
  }
  return context;
}
