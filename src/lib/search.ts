// Strips everything but letters/numbers (so punctuation, apostrophes —
// straight or curly — ellipses, etc. can't cause a mismatch) and
// lowercases, applied to both the query and the text being searched.
export function normalize(text: string): string {
  return text.toLowerCase().replace(/[^\p{L}\p{N}]+/gu, "");
}

// Returns 0 for an exact match, 1 for a prefix match, 2 for a substring
// match, or Infinity when the query doesn't appear in the text at all.
// Lower scores sort first so exact matches rise to the top.
export function matchScore(normalizedText: string, normalizedQuery: string): number {
  if (normalizedText === normalizedQuery) return 0;
  if (normalizedText.startsWith(normalizedQuery)) return 1;
  if (normalizedText.includes(normalizedQuery)) return 2;
  return Infinity;
}
