/**
 * FetchMetadataCommand -- encapsulates the validate-then-fetch pipeline for
 * Anyflip document metadata.
 *
 * Given a URL, this module:
 * 1. Validates the URL format via the Rust `validate_url` command.
 * 2. If valid, fetches `config.js` from the Anyflip CDN via `fetch_document_metadata`.
 * 3. Returns a `DocumentMetadata` object with title, page count, and page filenames.
 *
 * @module lib/ipc/metadata
 */

import { tauriInvoke, TauriCommandError } from './client';
import type { DocumentMetadata } from './client';

/**
 * Fetches metadata for an Anyflip document.
 *
 * Pipeline:
 * 1. Validate URL format (Rust: `validate_url`)
 * 2. Fetch config.js from Anyflip CDN (Rust: `fetch_document_metadata`)
 * 3. Parse and return `DocumentMetadata`
 *
 * @param url - Anyflip document URL (e.g., `https://anyflip.com/user/book`).
 * @returns DocumentMetadata with title, pageCount, and pageFilenames.
 * @throws {TauriCommandError} With code `INVALID_URL` if the URL is malformed.
 * @throws {TauriCommandError} With code `METADATA_ERROR` if fetch/parse fails.
 * @throws {TauriCommandError} With code `NETWORK_ERROR` if the HTTP request fails.
 *
 * @example
 * ```ts
 * const metadata = await fetchDocumentMetadata('https://anyflip.com/user/book');
 * console.log(metadata.title, metadata.pageCount);
 * ```
 */
export async function fetchDocumentMetadata(url: string): Promise<DocumentMetadata> {
  // Step 1: Validate URL format
  const isValid = await tauriInvoke<boolean>('validate_url', { url });
  if (!isValid) {
    throw new TauriCommandError(
      'INVALID_URL',
      `Invalid Anyflip URL: ${url}`,
      false,
      null
    );
  }

  // Step 2: Fetch metadata from Anyflip CDN
  const metadata = await tauriInvoke<DocumentMetadata>('fetch_document_metadata', { url });
  return metadata;
}
