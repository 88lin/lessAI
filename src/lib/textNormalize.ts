export function normalizeNewlines(text: string) {
  return text.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
}

export const normalizeLineEndingsToLf = normalizeNewlines;
