/**
 * Format a raw date string for display in the UI.
 *
 * Anyflip's config.js may return dates in several shapes:
 * - ISO 8601: `"2024-08-15"`, `"2024-08-15T10:30:00Z"`
 * - Localized: `"August 15, 2024"`, `"Mar 3, 2022"`
 *
 * We try `Date.parse` first; if that fails, return the input verbatim (it's
 * likely already human-readable). The UI hides the date line on `null`.
 *
 * @param raw  The raw date string from the backend.
 * @returns A formatted string like "Aug 15, 2024", or `null` if the input is
 *          empty/whitespace or unparseable.
 */
export function formatDate(raw: string | null | undefined): string | null {
  if (!raw || typeof raw !== 'string') return null;
  const trimmed = raw.trim();
  if (!trimmed) return null;

  // Try ISO/standard parsing first.
  const ms = Date.parse(trimmed);
  if (!Number.isNaN(ms)) {
    return new Date(ms).toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
    });
  }

  // Fall back: maybe the string is already a localized form like
  // "August 15, 2024". Validate it round-trips through Date.
  const probe = new Date(trimmed);
  if (!Number.isNaN(probe.getTime())) {
    return probe.toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
    });
  }

  // Last resort: return the original string if it has at least one digit and
  // some letters, otherwise give up.
  if (/\d/.test(trimmed) && /[A-Za-z]/.test(trimmed)) {
    return trimmed;
  }

  return null;
}
