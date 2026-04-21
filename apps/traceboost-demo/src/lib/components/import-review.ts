export type ImportConfirmationStage = string;

export interface ImportReviewItem {
  severity: "blocking" | "warning" | "info";
  title: string;
  message: string;
}

export interface ImportReviewField {
  label: string;
  value: string;
}

export interface ImportReviewSection {
  title: string;
  fields: ImportReviewField[];
  emptyMessage?: string;
  wide?: boolean;
}

export interface ImportFlowStep {
  key: ImportConfirmationStage;
  label: string;
  description: string;
  disabled?: boolean;
  status?: "pending" | "active" | "completed" | "warning" | "blocking";
  detail?: string;
}

export function compactImportReviewFields(
  entries: Array<[label: string, value: string | null | undefined]>
): ImportReviewField[] {
  return entries
    .map(([label, value]) => [label, typeof value === "string" ? value.trim() : value] as const)
    .filter((entry): entry is readonly [string, string] => !!entry[1])
    .map(([label, value]) => ({ label, value }));
}
