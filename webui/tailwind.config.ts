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
        
        background: "var(--color-background)",
        foreground: "var(--color-foreground)",
        
        
        
        
        
        card: {
          DEFAULT: "var(--color-surface)",
          foreground: "var(--color-on-surface)",
        },
        popover: {
          DEFAULT: "var(--color-popover)",
          foreground: "var(--color-on-surface)",
        },
        primary: {
          DEFAULT: "var(--color-primary)",
          foreground: "var(--color-secondary)",
        },
        secondary: {
          DEFAULT: "var(--color-surface-elevated)",
          foreground: "var(--color-on-surface)",
        },
        muted: {
          DEFAULT: "var(--color-muted)",
          foreground: "var(--color-muted-foreground)",
        },
        accent: {
          DEFAULT: "var(--color-accent)",
          foreground: "var(--color-accent-foreground)",
        },
        destructive: {
          DEFAULT: "var(--color-destructive)",
          foreground: "var(--color-destructive-foreground)",
        },
        border: "var(--color-border)",
        input: "var(--color-input)",
        ring: "var(--color-ring)",
        
        link: "var(--color-link)",
        success: "var(--color-success)",
        error: "var(--color-error)",
      },
    },
  },
  plugins: [require("tailwindcss-animate")],
};

export default config;
