module.exports = {
  title: "Solana Program Library Docs",
  tagline:
    "Solana is an open source project implementing a new, high-performance, permissionless blockchain.",
  url: "https://spl.solana.com",
  baseUrl: "/",
  favicon: "img/favicon.ico",
  organizationName: "solana-labs", // Usually your GitHub org/user name.
  projectName: "solana-program-library", // Usually your repo name.
  themeConfig: {
    navbar: {
      logo: {
        alt: "Solana Logo",
        src: "img/logo-horizontal.svg",
        srcDark: "img/logo-horizontal-dark.svg",
      },
      links: [
        {
          href: "https://docs.solana.com/",
          label: "Docs »",
          position: "left",
        },
        {
          href: "https://discordapp.com/invite/pquxPsq",
          label: "Chat",
          position: "right",
        },

        {
          href: "https://github.com/solana-labs/solana-program-library",
          label: "GitHub",
          position: "right",
        },
      ],
    },
    footer: {
      style: "dark",
      links: [
        {
          title: "Community",
          items: [
            {
              label: "Discord",
              href: "https://discordapp.com/invite/pquxPsq",
            },
            {
              label: "Twitter",
              href: "https://twitter.com/solana",
            },
            {
              label: "Forums",
              href: "https://forums.solana.com",
            },
          ],
        },
        {
          title: "More",
          items: [
            {
              label: "GitHub",
              href: "https://github.com/solana-labs/solana-program-library",
            },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Solana Foundation`,
    },
  },
  plugins: [require.resolve('docusaurus-lunr-search')],
  presets: [
    [
      "@docusaurus/preset-classic",
      {
        docs: {
          path: "src",
          routeBasePath: "/",
          homePageId: "introduction",
          sidebarPath: require.resolve("./sidebars.js"),
        },
        theme: {
          customCss: require.resolve("./src/css/custom.css"),
        },
      },
    ],
  ],
};
