import { createTheme, type Theme } from "@mui/material/styles";
import { PALETTE, withAlpha } from "./palette";

/**
 * Every focusable control must get a visible focus ring using only palette colors, and
 * never lose it to a browser-default `outline: none` reset — MUI components below opt
 * into `:focus-visible` explicitly.
 */
const focusRing = (mode: "dark" | "light") => ({
  outline: "none",
  boxShadow: `inset 0 0 0 2px ${mode === "dark" ? PALETTE.white : PALETTE.deepInk}`,
});

export function buildTheme(mode: "dark" | "light"): Theme {
  const isDark = mode === "dark";

  return createTheme({
    palette: {
      mode,
      background: {
        default: isDark ? PALETTE.deepInk : PALETTE.white,
        paper: isDark ? PALETTE.slate : PALETTE.lightGrey,
      },
      text: {
        primary: isDark ? PALETTE.white : PALETTE.deepInk,
        secondary: isDark ? PALETTE.warmSand : PALETTE.mutedMauve,
      },
      primary: {
        main: isDark ? PALETTE.warmSand : PALETTE.mutedMauve,
        contrastText: isDark ? PALETTE.deepInk : PALETTE.white,
      },
      secondary: {
        main: PALETTE.mutedMauve,
        contrastText: PALETTE.white,
      },
      divider: isDark ? withAlpha(PALETTE.warmSand, 0.16) : withAlpha(PALETTE.deepInk, 0.12),
    },
    typography: {
      fontFamily: ['"Inter"', "system-ui", "-apple-system", "sans-serif"].join(", "),
    },
    shape: {
      borderRadius: 12,
    },
    components: {
      MuiButtonBase: {
        defaultProps: { disableRipple: false },
        styleOverrides: {
          root: {
            "&.Mui-focusVisible": focusRing(mode),
          },
        },
      },
      MuiButton: {
        styleOverrides: {
          root: { textTransform: "none", fontWeight: 600 },
        },
      },
      MuiListItemButton: {
        styleOverrides: {
          root: {
            borderRadius: 10,
            "&.Mui-focusVisible": focusRing(mode),
          },
        },
      },
      MuiIconButton: {
        styleOverrides: {
          root: {
            "&.Mui-focusVisible": focusRing(mode),
          },
        },
      },
      MuiPaper: {
        styleOverrides: {
          root: { backgroundImage: "none" },
        },
      },
      MuiOutlinedInput: {
        styleOverrides: {
          // MUI animates the notch's `<legend>` width open/closed as the label shrinks —
          // over this app's glass/blur surfaces that transition leaves a visible gap in the
          // border for a frame or two instead of snapping shut cleanly. Removing the
          // transition makes it instant instead of animated, which avoids the artifact
          // entirely without changing the final open/closed states themselves.
          notchedOutline: { "& legend": { transition: "none" } },
        },
      },
      MuiAlert: {
        // `ToastProvider` renders `<Alert variant="filled">` for every toast — without this
        // override MUI's stock green/red/blue leaks through, violating the locked palette.
        // `styleOverrides` only exposes composite classes like `filled`/`colorSuccess`
        // separately in this MUI version (no combined `filledSuccess` class), so the
        // variant+color pairing is targeted via `variants`.
        variants: [
          {
            props: { variant: "filled", color: "success" },
            style: { backgroundColor: PALETTE.warmSand, color: PALETTE.deepInk },
          },
          {
            props: { variant: "filled", color: "error" },
            style: { backgroundColor: PALETTE.mutedMauve, color: PALETTE.white },
          },
          {
            props: { variant: "filled", color: "info" },
            style: {
              backgroundColor: isDark ? PALETTE.slate : PALETTE.lightGrey,
              color: isDark ? PALETTE.white : PALETTE.deepInk,
            },
          },
        ],
      },
      MuiCssBaseline: {
        styleOverrides: {
          "html, body, #root": {
            height: "100%",
            margin: 0,
            backgroundColor: "transparent",
          },
          ":focus-visible": focusRing(mode),
          // Text inputs/selects/textareas already show MUI's own built-in focus affordance
          // (the outlined notch thickens and recolors via `.Mui-focused`) on the control's
          // full visible box. The native-element focus target is just the inner `<input>`,
          // which is inset within that box — painting the generic ring there too draws a
          // second, smaller rectangle that visibly overlaps the label/border instead of
          // tracing the real control, so it's suppressed here in favor of that built-in style.
          "input:focus-visible, textarea:focus-visible, select:focus-visible": {
            outline: "none",
            boxShadow: "none",
          },
        },
      },
    },
  });
}
