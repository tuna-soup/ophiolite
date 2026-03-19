import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

export default defineConfig({
  site: "https://lithos.dev",
  integrations: [
    starlight({
      title: "Lithos",
      description:
        "Rust-first subsurface well-data SDK for logs, projects, structured wellbore assets, and typed compute.",
      social: {
        github: "https://github.com/scrooijmans/lithos",
      },
      editLink: {
        baseUrl:
          "https://github.com/scrooijmans/lithos/edit/master/apps/lithos-docs/",
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
            { label: "First Log", link: "/getting-started/first-log/" },
            {
              label: "First Project",
              link: "/getting-started/first-project/",
            },
          ],
        },
        {
          label: "Concepts",
          items: [
            { label: "Overview", link: "/concepts/overview/" },
            {
              label: "Well Domain Model",
              link: "/concepts/well-domain-model/",
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
              label: "Log Packages",
              link: "/packages-and-projects/log-packages/",
            },
            {
              label: "LithosProject",
              link: "/packages-and-projects/lithos-project/",
            },
          ],
        },
        {
          label: "Editing and Versioning",
          items: [
            { label: "Log Editing", link: "/editing-and-versioning/log-editing/" },
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
          label: "Architecture",
          items: [
            { label: "Overview", link: "/architecture/overview/" },
            { label: "ADRs", link: "/architecture/adrs/" },
          ],
        },
        {
          label: "Reference",
          items: [
            { label: "Core Types", link: "/reference/core-types/" },
            { label: "CLI", link: "/reference/cli/" },
            { label: "Harness", link: "/reference/harness/" },
          ],
        },
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
