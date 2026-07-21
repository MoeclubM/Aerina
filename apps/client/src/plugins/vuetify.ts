import { createVuetify } from "vuetify";
import { md3 } from "vuetify/blueprints";
import { aliases } from "vuetify/iconsets/mdi-svg";
import { aerinaIconSet } from "./icons";

export function createAppVuetify(defaultTheme: "light" | "dark") {
  return createVuetify({
    blueprint: md3,
    theme: {
      defaultTheme,
      themes: {
        light: {
          dark: false,
          colors: {
            primary: "#1565FF",
            secondary: "#64748B",
            tertiary: "#7A5CFF",
            surface: "#FFFFFF",
            "surface-bright": "#FFFFFF",
            "surface-light": "#F8FAFC",
            "surface-variant": "#F1F5F9",
            background: "#F1F3F8",
            "on-background": "#0F172A",
            "on-surface": "#0F172A",
            "on-surface-variant": "#64748B",
            "on-primary": "#FFFFFF",
            error: "#EF4444",
            info: "#0EA5E9",
            success: "#10B981",
            warning: "#F59E0B",
            outline: "#E2E8F0",
          },
        },
        dark: {
          dark: true,
          colors: {
            primary: "#4D8DFF",
            secondary: "#A0AEC0",
            tertiary: "#C4B5FD",
            surface: "#12151C",
            "surface-bright": "#191D27",
            "surface-light": "#222836",
            "surface-variant": "#2A3140",
            background: "#0B0D12",
            "on-background": "#F4F7FB",
            "on-surface": "#F4F7FB",
            "on-surface-variant": "#A0AEC0",
            "on-primary": "#081018",
            error: "#F87171",
            info: "#38BDF8",
            success: "#34D399",
            warning: "#FBBF24",
            outline: "#2C3342",
          },
        },
      },
    },
    icons: {
      defaultSet: "aerina",
      aliases,
      sets: {
        aerina: aerinaIconSet,
      },
    },
    defaults: {
      global: { ripple: false },
      VBtn: {
        rounded: "lg",
        elevation: 0,
        color: undefined,
        style: "text-transform: none; letter-spacing: 0; font-weight: 560;",
      },
      VCard: { rounded: "lg", elevation: 0 },
      VTextField: { variant: "outlined", density: "compact", hideDetails: "auto" },
      VSelect: { variant: "outlined", density: "compact", hideDetails: "auto" },
      VTextarea: { variant: "outlined", density: "compact", hideDetails: "auto" },
      VChip: { rounded: "pill", size: "small" },
      VList: { density: "compact", nav: true },
      VListItem: { rounded: "lg" },
      VBtnToggle: { rounded: "lg" },
      VMenu: { offset: 8 },
      VTooltip: { location: "end" },
      VDialog: { scrim: true },
      VNavigationDrawer: { elevation: 0 },
      VAppBar: { elevation: 0 },
    },
  });
}
