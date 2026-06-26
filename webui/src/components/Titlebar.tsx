interface TitlebarProps {
  /** Right-side actions cluster (settings / token usage / theme / add credential). */
  rightActions?: React.ReactNode;
}

export const Titlebar = ({ rightActions }: TitlebarProps) => {
  return (
    <header
      className="fixed top-0 left-0 right-0 z-50 flex items-center justify-between px-4 border-b border-border bg-card"
      style={{ height: 48 }}
      data-density
    >
      {/* Left: Serif KeyPilot wordmark */}
      <div className="flex items-center">
        <span
          className="font-serif font-medium text-[var(--color-neutral)]"
          style={{ fontSize: "var(--font-size-lg)", letterSpacing: "var(--tracking-tight)" }}
        >
          KeyPilot
        </span>
      </div>

      {/* Right: actions cluster (TopRightActions) */}
      <div className="flex items-center">{rightActions}</div>
    </header>
  );
};
