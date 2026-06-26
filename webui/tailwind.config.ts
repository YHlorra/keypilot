import type { Config } from "tailwindcss";

const config: Config = {
  darkMode: ["selector", "[data-theme='dark']"],
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        serif: ["var(--font-serif)"],
        sans: ["var(--font-sans)"],
        mono: ["var(--font-mono)"],
      },
      spacing: {
        "space-xs": "var(--spacing-xs)",
        "space-sm": "var(--spacing-sm)",
        "space-md": "var(--spacing-md)",
        "space-lg": "var(--spacing-lg)",
        "space-xl": "var(--spacing-xl)",
      },
      borderRadius: {
        sm: "var(--radius-sm)",
        pill: "var(--radius-pill)",
      },
      colors: {
        /* shadcn-compatible base */
        background: "var(--color-background)",
        foreground: "var(--color-foreground)",
        card: "var(--color-surface)",
        cardForeground: "var(--color-on-surface)",
        primary: "var(--color-primary)",
        primaryForeground: "var(--color-secondary)",
        secondary: "var(--color-surface-elevated)",
        secondaryForeground: "var(--color-on-surface)",
        muted: "var(--color-muted)",
        "muted-foreground": "var(--color-muted-foreground)",
        accent: "var(--color-accent)",
        accentForeground: "var(--color-accent-foreground)",
        destructive: "var(--color-destructive)",
        destructiveForeground: "var(--color-destructive-foreground)",
        border: "var(--color-border)",
        input: "var(--color-input)",
        ring: "var(--color-ring)",
        /* Kaku extras */
        link: "var(--color-link)",
        success: "var(--color-success)",
        error: "var(--color-error)",
      },
    },
  },
  plugins: [require("tailwindcss-animate")],
};

export default config;
