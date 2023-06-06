/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    './src/pages/**/*.{js,ts,jsx,tsx,mdx}',
    './src/components/**/*.{js,ts,jsx,tsx,mdx}',
    './src/app/**/*.{js,ts,jsx,tsx,mdx}',
  ],
  theme: {
    extend: {
      colors: {
        primary: "#38BDF8",
        black: "#000000",
        neutral: {
          50: "#FAFAFA",
          300: "#D4D4D4",
          400: "#A3A3A3",
          500: "#737373",
          700: "#404040",
          800: "#262626",
          900: "#171717",
        }
      }
    },
  },
  plugins: [],
}
