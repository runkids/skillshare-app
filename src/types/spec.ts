/**
 * Spec type definitions
 * Matches the Rust Spec struct serialized via serde + manual JSON additions.
 *
 * Note: `create_spec`, `get_spec`, `update_spec` return this shape (snake_case keys).
 * `list_specs` returns SpecRow (camelCase keys) — see SpecListItem below.
 */

/** Full Spec returned by get_spec / create_spec / update_spec */
export interface Spec {
  id: string;
  schema: string;
  title: string;
  status: string;
  workflow?: string;
  workflow_phase?: string;
  created_at: string;
  updated_at: string;
  fields: Record<string, unknown>;
  body: string;
  filePath?: string;
}

/**
 * Row returned by `list_specs` (from SpecRow with `#[serde(rename_all = "camelCase")]`).
 * Uses camelCase field names matching the Rust SpecRow serialization.
 */
export interface SpecListItem {
  id: string;
  schemaId: string;
  title: string;
  status: string;
  workflowId?: string | null;
  workflowPhase?: string | null;
  filePath: string;
  fieldsJson?: string | null;
  createdAt: string;
  updatedAt: string;
}
