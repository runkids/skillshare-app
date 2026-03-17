/**
 * Schema type definitions
 * Matches the Rust SchemaDefinition / FieldType / SectionDef structs.
 *
 * FieldType is serialized as `#[serde(untagged)]`:
 *   - Simple type: a plain string like "string", "number", "date", "list"
 *   - Enum type: an array of allowed string values, e.g. ["high", "medium", "low"]
 */

export type FieldType = string | string[];

export interface SectionDef {
  name: string;
  required: boolean;
}

export interface SchemaDefinition {
  name: string;
  display_name?: string;
  description?: string;
  fields: Record<string, FieldType>;
  sections: SectionDef[];
}
