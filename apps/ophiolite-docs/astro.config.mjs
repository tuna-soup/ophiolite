import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

export default defineConfig({
  site: "https://ophiolite.dev",
  integrations: [
    starlight({
      title: "Ophiolite",
      description:
        "Local-first subsurface infrastructure with a canonical Rust runtime and Python, CLI, and Rust surfaces.",
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/scrooijmans/ophiolite",
        },
      ],
      editLink: {
        baseUrl:
          "https://github.com/scrooijmans/ophiolite/edit/master/apps/ophiolite-docs/",
      },
      disable404Route: true,
      customCss: ["./src/styles/custom.css"],
      sidebar: [
        { label: "Home", link: "/" },
        {
          label: "Docs",
          items: [
            { label: "Overview", link: "/docs/" },
            {
              label: "Start Here",
              collapsed: true,
              autogenerate: { directory: "docs/start-here" },
            },
            {
              label: "Build Workflows",
              collapsed: true,
              autogenerate: { directory: "docs/build-workflows" },
            },
            {
              label: "Core Concepts",
              collapsed: true,
              autogenerate: { directory: "docs/core-concepts" },
            },
            {
              label: "Data Model",
              collapsed: true,
              autogenerate: { directory: "docs/data-model" },
            },
            {
              label: "Embed in Apps",
              collapsed: true,
              autogenerate: { directory: "docs/embed-in-apps" },
            },
            {
              label: "Advanced",
              collapsed: true,
              autogenerate: { directory: "docs/advanced" },
            },
          ],
        },
        {
          label: "Operators",
          items: [
            { label: "Overview", link: "/operators/" },
            { label: "Write Your First Operator", slug: "operators/write-your-first-operator" },
            { label: "Package and Install", slug: "operators/package-and-install" },
            { label: "Testing and Debugging", slug: "operators/testing-and-debugging" },
          ],
        },
        {
          label: "Examples",
          items: [
            { label: "Overview", link: "/examples/" },
            { label: "Python Project Workflow", slug: "examples/python-project-workflow" },
            { label: "CLI Project Automation", slug: "examples/cli-project-automation" },
            { label: "Custom Python Operator", slug: "examples/custom-python-operator" },
          ],
        },
        {
          label: "Reference",
          items: [
            { label: "Overview", link: "/reference/" },
            { label: "Python SDK", slug: "reference/python-sdk" },
            { label: "CLI", slug: "reference/cli" },
            { label: "Operator Packages", slug: "reference/operator-packages" },
            { label: "Contracts and Schemas", slug: "reference/contracts-and-schemas" },
            { label: "Surface Matrix", slug: "reference/surface-matrix" },
          ],
        },
        { label: "Changelog", link: "/changelog/" },
      ],
    }),
  ],
});
