/**
 * The bullet points of a Release Please changelog body, as plain text.
 * Links keep their text, `**bold**` loses its markers, and trailing
 * issue and commit references like `(#89)` or `(291d40a)` are dropped.
 */
export function releaseNoteItems(body: string | null | undefined): string[] {
  if (!body) return [];
  return body
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => /^[*-]\s+/.test(line))
    .map((line) =>
      line
        .replace(/^[*-]\s+/, '')
        .replace(/\[([^\]]*)\]\([^)]*\)/g, '$1')
        .replace(/\*\*/g, '')
        .replace(/(\s*\((#\d+|[0-9a-f]{7,40})\))+$/g, '')
        .trim()
    )
    .filter(Boolean);
}
