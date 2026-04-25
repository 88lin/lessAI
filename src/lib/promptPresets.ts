import aigcV1Prompt from "../../prompt/1.txt?raw";
import humanizerZhPrompt from "../../prompt/2.txt?raw";
import thesisAiReductionPrompt from "../../prompt/4.txt?raw";
import type { PromptPresetId } from "./types";

export interface PromptPresetDefinition {
  id: PromptPresetId;
  label: string;
  hint: string;
  content: string;
}

export const PROMPT_PRESETS: ReadonlyArray<PromptPresetDefinition> = [
  {
    id: "aigc_v1",
    label: "方案 1：论文 / 技术文档改写",
    hint: "更偏“解释性 + 写作展开”，尽量保持原意与字数接近",
    content: aigcV1Prompt
  },
  {
    id: "humanizer_zh",
    label: "方案 2：Humanizer-ZH",
    hint: "更偏“去 AI 痕迹”，减少模板化表达与空洞修饰",
    content: humanizerZhPrompt
  },
  {
    id: "thesis_ai_reduction",
    label: "方案 3：学术自然优化",
    hint: "更偏学术论文人性化改写，降低机器感，同时尽量保留原有信息密度",
    content: thesisAiReductionPrompt
  }
] as const;

export function makePromptPreview(content: string, maxChars = 320) {
  const normalized = content.trim().replace(/\r\n/g, "\n").replace(/\r/g, "\n");
  const compact = normalized.replace(/\n{3,}/g, "\n\n");
  if (compact.length <= maxChars) return compact;
  return `${compact.slice(0, maxChars)}...`;
}
