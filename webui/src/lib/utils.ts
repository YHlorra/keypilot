import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";
import type { Category } from "@/types/api";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function isLlmCategory(categoryId: number, categories: Category[]): boolean {
  const cat = categories.find((c) => c.id === categoryId);
  return !!cat && cat.name.trim().toLowerCase() === "llm";
}
