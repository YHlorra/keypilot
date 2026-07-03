interface TitlebarProps {
  
  rightActions?: React.ReactNode;
}

export const Titlebar = ({ rightActions }: TitlebarProps) => {
  
  
  
  return (
    <header
      className="shrink-0 flex items-center justify-between px-4 border-b border-border bg-card"
      style={{ height: 48 }}
      data-density
    >
      {}
      <div className="flex items-center">
        <span
          className="font-serif font-medium text-[var(--color-neutral)]"
          style={{ fontSize: "var(--font-size-lg)", letterSpacing: "var(--tracking-tight)" }}
        >
          KeyPilot
        </span>
      </div>

      {}
      <div className="flex items-center">{rightActions}</div>
    </header>
  );
};
