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
        // Kaku tokens (shadcn naming retained, values switched)
        background: "var(--color-background)",
        foreground: "var(--color-foreground)",
        card: "var(--color-surface)",
        cardForeground: "var(--color-on-surface)",
        primary: "var(--color-primary)",
        primaryForeground: "var(--color-secondary)",
        border: "var(--color-border)",
        ring: "var(--color-primary)",
        // new Kaku tokens
        link: "var(--color-link)",
        success: "var(--color-success)",
      },
    },
  },
  plugins: [require("tailwindcss-animate")],
};

export default config;
