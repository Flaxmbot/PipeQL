/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        "secondary-fixed-dim": "#ffb4a9",
        "on-tertiary": "#ffffff",
        "on-background": "#171c20",
        "surface": "#f6faff",
        "surface-container": "#eaeef4",
        "on-secondary-fixed-variant": "#930004",
        "background": "#f6faff",
        "on-tertiary-fixed-variant": "#5c4300",
        "secondary": "#b51b15",
        "surface-tint": "#005ac1",
        "outline-variant": "#c2c6d5",
        "primary": "#0058bd",
        "tertiary": "#765700",
        "tertiary-fixed-dim": "#fbbc06",
        "surface-variant": "#dee3e8",
        "on-primary-fixed": "#001a41",
        "inverse-on-surface": "#edf1f7",
        "on-secondary-fixed": "#410001",
        "secondary-container": "#d9372b",
        "surface-container-lowest": "#ffffff",
        "on-error-container": "#93000a",
        "surface-dim": "#d6dae0",
        "on-surface": "#171c20",
        "on-secondary": "#ffffff",
        "inverse-surface": "#2c3135",
        "on-surface-variant": "#424753",
        "on-secondary-container": "#fffbff",
        "on-primary-fixed-variant": "#004494",
        "surface-container-high": "#e4e9ee",
        "on-error": "#ffffff",
        "secondary-fixed": "#ffdad5",
        "outline": "#727785",
        "primary-fixed": "#d8e2ff",
        "surface-bright": "#f6faff",
        "on-primary-container": "#fefcff",
        "on-primary": "#ffffff",
        "surface-container-highest": "#dee3e8",
        "primary-container": "#2771df",
        "primary-fixed-dim": "#adc6ff",
        "on-tertiary-fixed": "#261a00",
        "surface-container-low": "#f0f4fa",
        "on-tertiary-container": "#fffbff",
        "tertiary-container": "#956e00",
        "tertiary-fixed": "#ffdea0",
        "error-container": "#ffdad6",
        "error": "#ba1a1a",
        "inverse-primary": "#adc6ff"
      },
      borderRadius: {
        "DEFAULT": "1rem",
        "lg": "2rem",
        "xl": "3rem",
        "full": "9999px"
      },
      spacing: {
        "margin-desktop": "64px",
        "base": "8px",
        "margin-mobile": "16px",
        "gutter": "24px",
        "container-max": "1280px"
      },
      fontFamily: {
        "sans": ["Inter", "sans-serif"],
        "mono": ["JetBrains Mono", "monospace"]
      }
    }
  },
  plugins: [
    require('@tailwindcss/forms'),
    require('@tailwindcss/container-queries'),
  ],
}
