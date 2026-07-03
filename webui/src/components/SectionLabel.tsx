import { cn } from "@/lib/utils";

interface SectionLabelProps {
  children: React.ReactNode;
}

export const SectionLabel = ({ children }: SectionLabelProps) => {
  return (
    <div
      className={cn(
        "px-7 pt-6 pb-2 font-serif",
        "text-[var(--color-primary)]",
        "font-semibold uppercase"
      )}
      style={{ fontSize: "var(--font-size-xs)", letterSpacing: "var(--tracking-wider)" }}
    >
      {children}
    </div>
  );
};