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

// Simple UUID v4 generator
function generateUUID(): string {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = Math.random() * 16 | 0;
    const v = c === 'x' ? r : (r & 0x3 | 0x8);
    return v.toString(16);
  });
}

// Parse a single entry from lines
function parseEntry(headerLine: string, contentLines: string[]): Document | null {
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

  return {
    id: generateUUID(),
    metadata: {
      title: title.trim(),
      date: date.toISOString(),
      tags,
    },
    text,
  };
}

// Parse the entire changelog content
function parseChangelog(content: string): Document[] {
  const lines = content.split('\n');
  const documents: Document[] = [];

  let currentHeader: string | null = null;
  let currentContent: string[] = [];

  for (const line of lines) {
    if (line.startsWith('* ') && /\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}/.test(line)) {
      // Save previous entry if exists
      if (currentHeader) {
        const doc = parseEntry(currentHeader, currentContent);
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
    const doc = parseEntry(currentHeader, currentContent);
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
  const documents = parseChangelog(content);

  // Output as JSONL to stdout
  for (const doc of documents) {
    console.log(JSON.stringify(doc));
  }

  // Print stats to stderr
  console.error(`Converted ${documents.length} documents`);
}

main();
