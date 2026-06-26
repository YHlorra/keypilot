import { cn } from "@/lib/utils";

interface SectionLabelProps {
  children: React.ReactNode;
}

export const SectionLabel = ({ children }: SectionLabelProps) => {
  return (
    <div
      className={cn(
        "px-7 pt-6 pb-2",
        "text-xs font-normal uppercase tracking-[0.06em]",
        "text-[var(--color-primary)]",
        "font-serif"
      )}
    >
      {children}
    </div>
  );
};