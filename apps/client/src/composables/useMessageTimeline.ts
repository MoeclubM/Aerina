import type { MessageView } from "../api";

export type TimelineItem =
  | { kind: "user"; key: string; message: MessageView }
  | { kind: "assistant"; key: string; message: MessageView }
  | {
      kind: "round";
      key: string;
      parentId: string;
      roundId?: string | null;
      selectedCandidateId?: string | null;
      messages: MessageView[];
    };

export function buildTimeline(
  messages: MessageView[],
  selectedByRound: Record<string, string | null | undefined> = {},
): TimelineItem[] {
  const items: TimelineItem[] = [];
  let i = 0;
  while (i < messages.length) {
    const m = messages[i];
    if (m.message.role === "user") {
      items.push({ kind: "user", key: m.message.id, message: m });
      const parentId = m.message.id;
      const assistants: MessageView[] = [];
      let j = i + 1;
      while (
        j < messages.length &&
        messages[j].message.role === "assistant" &&
        messages[j].message.parent_message_id === parentId
      ) {
        assistants.push(messages[j]);
        j += 1;
      }
      if (assistants.length > 1) {
        const roundId = assistants.find((a) => a.message.round_id)?.message.round_id ?? null;
        items.push({
          kind: "round",
          key: `round:${parentId}`,
          parentId,
          roundId,
          selectedCandidateId: roundId ? selectedByRound[roundId] ?? null : null,
          messages: assistants,
        });
      } else if (assistants.length === 1) {
        items.push({
          kind: "assistant",
          key: assistants[0].message.id,
          message: assistants[0],
        });
      }
      i = j;
      continue;
    }
    items.push({ kind: "assistant", key: m.message.id, message: m });
    i += 1;
  }
  return items;
}

export function textOf(message: MessageView): string {
  return message.blocks
    .filter((b) => b.type === "text" && b.text)
    .map((b) => b.text as string)
    .join("\n");
}

export function usageOf(message: MessageView) {
  return message.blocks.find((b) => b.type === "usage_meta");
}

export function imagesOf(message: MessageView) {
  return message.blocks.filter((b) => b.type === "image" && b.media_id);
}

export function thinkingOf(message: MessageView): string {
  return message.blocks
    .filter((b) => b.type === "thinking" && b.text)
    .map((b) => b.text as string)
    .join("\n");
}

export function thinkingMetaOf(message: MessageView): { tokens?: number; durationMs?: number } {
  const block = message.blocks.find((b) => b.type === "thinking");
  return {
    tokens: block?.reasoning_tokens,
    durationMs: block?.reasoning_duration_ms,
  };
}
