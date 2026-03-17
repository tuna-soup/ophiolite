import { invoke } from "@tauri-apps/api/core";

export type CommandErrorDto = {
  kind: string;
  message: string;
  session_id?: string | null;
  validation?: unknown;
  save_conflict?: unknown;
};

export type CommandResponse<T> = { Ok: T } | { Err: CommandErrorDto };

export type SessionSummaryDto = {
  session_id: string;
  root: string;
  revision: string;
  dirty: { has_unsaved_changes: boolean };
};

export type SessionMetadataDto = {
  session: {
    session_id: string;
    root: string;
    revision: string;
  };
  metadata: {
    metadata: {
      well: {
        company?: string | null;
      };
    };
  };
};

export async function invokeCommand<T>(
  command: string,
  request: unknown
): Promise<CommandResponse<T>> {
  return invoke(command, { request });
}
