import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

export default defineConfig({
  site: "https://ophiolitecharts.com",
  integrations: [
    starlight({
      title: "Ophiolite Charts",
      description:
        "High-performance scientific and subsurface charts for modern web apps, with live examples, typed interactions, and commercial support.",
      social: {
        github: "https://github.com/scrooijmans/ophiolite"
      },
      editLink: {
        baseUrl: "https://github.com/scrooijmans/ophiolite/edit/master/apps/ophiolitecharts-docs/"
      },
      disable404Route: true,
      customCss: ["./src/styles/custom.css"],
      sidebar: [
        { label: "Overview", link: "/docs/" },
        {
          label: "Getting Started",
          collapsed: true,
          autogenerate: { directory: "docs/getting-started" }
        },
        {
          label: "Core Concepts",
          collapsed: true,
          autogenerate: { directory: "docs/core-concepts" }
        },
        {
          label: "Chart Families",
          collapsed: true,
          autogenerate: { directory: "docs/chart-families" }
        },
        {
          label: "Data Models",
          collapsed: true,
          autogenerate: { directory: "docs/data-models" }
        },
        {
          label: "Interactions",
          collapsed: true,
          autogenerate: { directory: "docs/interactions" }
        },
        {
          label: "Embedding",
          collapsed: true,
          autogenerate: { directory: "docs/embedding" }
        }
      ]
    })
  ]
});
