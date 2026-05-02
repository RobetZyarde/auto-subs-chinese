import React from 'react';
import { SettingsProvider } from './SettingsContext';
import { ModelsProvider } from './ModelsContext';
import { ResolveProvider } from './ResolveContext';
import { TranscriptProvider } from './TranscriptContext';
import { ProgressProvider } from './ProgressContext';
import { PresetsProvider } from './PresetsContext';
import { ErrorDialogProvider } from './ErrorDialogContext';
import { IntegrationProvider } from './IntegrationContext';

import { PremiereProvider } from './PremiereContext';

interface GlobalProviderProps {
  children: React.ReactNode;
}

interface EditorWorkspaceProvidersProps {
  children: React.ReactNode;
}

/**
 * GlobalProvider - Composition wrapper that nests all context providers
 * 
 * Root provider nesting order (from outermost to innermost):
 * 1. SettingsProvider - Core settings and persistence (leaf context)
 * 2. ModelsProvider - Model management (depends on settings)
 * 3. IntegrationProvider - Active host app selection
 * 
 * Editor-specific providers are mounted by EditorWorkspaceProviders, closer to
 * the UI that actually needs host app state, transcript data, and progress.
 */
export function GlobalProvider({ children }: GlobalProviderProps) {
  // ErrorDialogProvider is mounted at the outermost layer so any inner
  // provider or component (transcription, Resolve calls, etc.) can reach the
  // single app-wide error dialog via `useErrorDialog()`.
  return (
    <ErrorDialogProvider>
      <SettingsProvider>
        <ModelsProvider>
          <IntegrationProvider>
            {children}
          </IntegrationProvider>
        </ModelsProvider>
      </SettingsProvider>
    </ErrorDialogProvider>
  );
}

export function EditorWorkspaceProviders({ children }: EditorWorkspaceProvidersProps) {
  return (
    <ResolveProvider>
      <PremiereProvider>
        <TranscriptProvider>
          <ProgressProvider>
            <PresetsProvider>
              {children}
            </PresetsProvider>
          </ProgressProvider>
        </TranscriptProvider>
      </PremiereProvider>
    </ResolveProvider>
  );
}
