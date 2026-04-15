import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

export default defineConfig({
  site: "https://ophiolite.dev",
  integrations: [
    starlight({
      title: "Ophiolite",
      description:
        "Rust-first subsurface core for canonical contracts, local-first runtime primitives, and app-boundary DTOs.",
      social: {
        github: "https://github.com/scrooijmans/ophiolite",
      },
      editLink: {
        baseUrl:
          "https://github.com/scrooijmans/ophiolite/edit/master/apps/ophiolite-docs/",
      },
      customCss: ["./src/styles/custom.css"],
      sidebar: [
        {
          label: "Start Here",
          items: [
            { label: "Home", link: "/" },
            {
              label: "Installation",
              link: "/getting-started/installation/",
            },
            { label: "First Asset", link: "/getting-started/first-asset/" },
            {
              label: "First Project",
              link: "/getting-started/first-project/",
            },
          ],
        },
        {
          label: "Products",
          items: [
            { label: "Platform Core", link: "/products/platform-core/" },
            { label: "Ophiolite Charts", link: "/products/ophiolite-charts/" },
          ],
        },
        {
          label: "Concepts",
          items: [
            { label: "Overview", link: "/concepts/overview/" },
            {
              label: "Platform Boundary",
              link: "/concepts/platform-boundary/",
            },
            {
              label: "Subsurface Domain Model",
              link: "/concepts/subsurface-domain-model/",
            },
            {
              label: "Package, Session, Workspace",
              link: "/concepts/package-session-workspace/",
            },
            { label: "Asset Families", link: "/concepts/asset-families/" },
            {
              label: "Revisions and History",
              link: "/concepts/revisions-and-history/",
            },
            {
              label: "Compute and Derived Assets",
              link: "/concepts/compute-and-derived-assets/",
            },
          ],
        },
        {
          label: "Packages and Projects",
          items: [
            {
              label: "Asset Packages",
              link: "/packages-and-projects/asset-packages/",
            },
            {
              label: "OphioliteProject",
              link: "/packages-and-projects/ophiolite-project/",
            },
          ],
        },
        {
          label: "Editing and Versioning",
          items: [
            { label: "Asset Editing", link: "/editing-and-versioning/asset-editing/" },
            {
              label: "Structured Editing",
              link: "/editing-and-versioning/structured-editing/",
            },
            {
              label: "Revision History",
              link: "/editing-and-versioning/revision-history/",
            },
          ],
        },
        {
          label: "Interfaces",
          items: [
            {
              label: "Contracts and DTOs",
              link: "/reference/contracts-and-dtos/",
            },
            { label: "CLI", link: "/reference/cli/" },
            {
              label: "Automation Surfaces",
              link: "/reference/automation-interfaces/",
            },
            { label: "Harness", link: "/reference/harness/" },
          ],
        },
        {
          label: "Architecture",
          items: [
            { label: "Overview", link: "/architecture/overview/" },
            { label: "Repo Structure", link: "/architecture/repo-structure/" },
            { label: "ADRs", link: "/architecture/adrs/" },
          ],
        },
        { label: "Reference", items: [{ label: "Core Types", link: "/reference/core-types/" }] },
        {
          label: "Examples",
          items: [
            {
              label: "Synthetic Project Fixture",
              link: "/examples/synthetic-project-fixture/",
            },
          ],
        },
      ],
    }),
  ],
});
