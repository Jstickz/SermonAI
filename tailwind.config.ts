import type { Config } from "tailwindcss";
import tokens from "./src/design/tokens.json";

// Branding §13. Never hard-code hex in components: use these token names.
export default {
  content: ["./index.html", "./projector.html", "./alternate.html", "./remote.html", "./src/**/*.{ts,tsx,html}"],
  theme: {
    extend: {
      colors: {
        bg: tokens.color.bg,
        line: tokens.color.border,
        content: tokens.color.text,
        accent: tokens.color.accent.violet,
        status: {
          success: tokens.color.status.success.fg,
          "success-bg": tokens.color.status.success.bg,
          warning: tokens.color.status.warning.fg,
          "warning-bg": tokens.color.status.warning.bg,
          danger: tokens.color.status.danger.fg,
          "danger-bg": tokens.color.status.danger.bg,
          info: tokens.color.status.info.fg,
          "info-bg": tokens.color.status.info.bg,
        },
        source: tokens.color.source,
        pdf: tokens.color.pdf,
      },
      borderRadius: {
        sm: `${tokens.radius.sm}px`,
        md: `${tokens.radius.md}px`,
        lg: `${tokens.radius.lg}px`,
        xl: `${tokens.radius.xl}px`,
        pill: `${tokens.radius.pill}px`,
      },
      fontFamily: {
        sans: tokens.type.family.ui.split(", "),
        display: tokens.type.family.display.split(", "),
        serif: tokens.type.family.scripture.split(", "),
        mono: tokens.type.family.mono.split(", "),
      },
      transitionTimingFunction: {
        "brand-out": tokens.motion["ease-out"],
        "brand-in-out": tokens.motion["ease-in-out"],
      },
      transitionDuration: {
        fast: "120ms",
        base: "180ms",
        "card-enter": "200ms",
        "verse-in": "400ms",
        "verse-out": "250ms",
      },
      boxShadow: {
        hover: "0 1px 0 0 rgba(255,255,255,0.04)",
        pop: "0 8px 24px -8px rgba(0,0,0,0.5)",
        modal: "0 24px 48px -12px rgba(0,0,0,0.6), 0 0 0 1px rgba(255,255,255,0.05)",
        "glow-violet": "0 0 24px -4px rgba(185,128,232,0.35)",
      },
    },
  },
  plugins: [],
} satisfies Config;
