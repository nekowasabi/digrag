#!/usr/bin/env -S deno run --allow-read --allow-write
/**
 * convert-changelogmemo.ts
 *
 * Converts changelogmemo format to JSONL format for digrag.
 *
 * Usage:
 *   deno run --allow-read scripts/convert-changelogmemo.ts < changelogmemo > output.jsonl
 *   cat changelogmemo | deno run --allow-read scripts/convert-changelogmemo.ts | digrag build --input - --output .rag
 *
 *   # Or with bun:
 *   bun scripts/convert-changelogmemo.ts < changelogmemo > output.jsonl
 *
 * Input format (changelogmemo):
 *   * Title 2025-01-15 10:00:00 [tag1]:[tag2]:
 *   Content line 1
 *   Content line 2
 *
 *   * Another Title 2025-01-14 09:00:00 [memo]:
 *   More content
 *
 * Output format (JSONL):
 *   {"id":"uuid","metadata":{"title":"Title","date":"2025-01-15T10:00:00Z","tags":["tag1","tag2"]},"text":"Content"}
 */

interface Document {
  id: string;
  metadata: {
    title: string;
    date: string;
    tags: string[];
  };
  text: string;
}

// Compute content hash from title and text
// Uses SHA256 hash of "title\0text" and returns first 16 hex characters.
// This ensures reproducible document IDs based on content only.
// (Matches Rust implementation in src/loader/document.rs)
async function computeContentHash(title: string, text: string): Promise<string> {
  const encoder = new TextEncoder();
  const data = encoder.encode(title + '\0' + text);
  const hashBuffer = await crypto.subtle.digest('SHA-256', data);
  const hashArray = new Uint8Array(hashBuffer);
  // First 8 bytes = 16 hex characters
  const hex = Array.from(hashArray.slice(0, 8))
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
  return hex;
}

// Parse a single entry from lines
async function parseEntry(headerLine: string, contentLines: string[]): Promise<Document | null> {
  // Pattern: * Title YYYY-MM-DD HH:MM:SS [tags]:
  const headerPattern = /^\* (.+?) (\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2})\s*(.*)$/;
  const match = headerLine.match(headerPattern);

  if (!match) {
    return null;
  }

  const [, title, dateStr, tagsStr] = match;

  // Parse date to ISO format
  const date = new Date(dateStr.replace(' ', 'T') + 'Z');

  // Extract tags from [tag]: pattern
  const tagPattern = /\[([^\]]+)\]:/g;
  const tags: string[] = [];
  let tagMatch;
  while ((tagMatch = tagPattern.exec(tagsStr)) !== null) {
    tags.push(tagMatch[1]);
  }

  // Join content lines
  const text = contentLines.join('\n').trim();
  const trimmedTitle = title.trim();

  // Compute content-based ID (matches Rust implementation)
  const id = await computeContentHash(trimmedTitle, text);

  return {
    id,
    metadata: {
      title: trimmedTitle,
      date: date.toISOString(),
      tags,
    },
    text,
  };
}

// Parse the entire changelog content
async function parseChangelog(content: string): Promise<Document[]> {
  const lines = content.split('\n');
  const documents: Document[] = [];

  let currentHeader: string | null = null;
  let currentContent: string[] = [];

  for (const line of lines) {
    if (line.startsWith('* ') && /\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}/.test(line)) {
      // Save previous entry if exists
      if (currentHeader) {
        const doc = await parseEntry(currentHeader, currentContent);
        if (doc) {
          documents.push(doc);
        }
      }

      // Start new entry
      currentHeader = line;
      currentContent = [];
    } else if (currentHeader) {
      // Add to current content
      currentContent.push(line);
    }
  }

  // Don't forget the last entry
  if (currentHeader) {
    const doc = await parseEntry(currentHeader, currentContent);
    if (doc) {
      documents.push(doc);
    }
  }

  return documents;
}

// Main function
async function main() {
  // Read from stdin or file argument
  let content: string;

  if (Deno.args.length > 0) {
    // Read from file
    content = await Deno.readTextFile(Deno.args[0]);
  } else {
    // Read from stdin
    const decoder = new TextDecoder();
    const chunks: Uint8Array[] = [];

    for await (const chunk of Deno.stdin.readable) {
      chunks.push(chunk);
    }

    const totalLength = chunks.reduce((acc, chunk) => acc + chunk.length, 0);
    const combined = new Uint8Array(totalLength);
    let offset = 0;
    for (const chunk of chunks) {
      combined.set(chunk, offset);
      offset += chunk.length;
    }

    content = decoder.decode(combined);
  }

  // Parse and output
  const documents = await parseChangelog(content);

  // Output as JSONL to stdout
  for (const doc of documents) {
    console.log(JSON.stringify(doc));
  }

  // Print stats to stderr
  console.error(`Converted ${documents.length} documents`);
}

main();
