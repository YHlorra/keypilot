import type { Config } from "tailwindcss";

const config: Config = {
  darkMode: ["class"],
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      // AGENTS.md §3.4: shadcn tokens MUST override to Radix Colors (not Tailwind defaults)
      colors: {
        // Chrome (Radix gray + iris accent)
        background: "var(--color-background)",
        foreground: "var(--color-foreground)",
        card: "var(--color-card)",
        cardForeground: "var(--color-card-foreground)",
        popover: "var(--color-popover)",
        popoverForeground: "var(--color-popover-foreground)",
        primary: "var(--color-primary)",
        primaryForeground: "var(--color-primary-foreground)",
        secondary: "var(--color-secondary)",
        secondaryForeground: "var(--color-secondary-foreground)",
        muted: "var(--color-muted)",
        mutedForeground: "var(--color-muted-foreground)",
        accent: "var(--color-accent)",
        accentForeground: "var(--color-accent-foreground)",
        destructive: "var(--color-destructive)",
        destructiveForeground: "var(--color-destructive-foreground)",
        border: "var(--color-border)",
        input: "var(--color-input)",
        ring: "var(--color-ring)",
        // Status colors (Radix grass / amber / red / ruby)
        success: "var(--color-success)",
        warning: "var(--color-warning)",
        danger: "var(--color-danger)",
        critical: "var(--color-critical)",
      },
      borderRadius: {
        lg: "var(--radius)",
        md: "calc(var(--radius) - 2px)",
        sm: "calc(var(--radius) - 4px)",
      },
    },
  },
  plugins: [require("tailwindcss-animate")],
};

export default config;
