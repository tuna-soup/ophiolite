# ophiolitecharts-docs

Public site source for `ophiolitecharts.com`.

This site is the product-facing home for Ophiolite Charts rather than the broader Ophiolite platform. It combines:

- live examples
- product documentation
- pricing and evaluation guidance
- benchmark methodology and evidence

## Local Development

```bash
bun install
bun run dev
```

## Build

```bash
bun run build
```

The build includes a bundled copy of the public Svelte examples runtime under `/live/`.

## Deployment

The site is built by `.github/workflows/charts-site.yml` and uploaded as a static artifact.

The intended production host is:

- `https://ophiolitecharts.com`

Required domain steps outside the repo:

1. Choose a separate static host or a separate repository-level Pages site for `ophiolitecharts.com`.
2. Point `ophiolitecharts.com` at that deployment target.
3. Add `www` as a redirect if you want a `www.ophiolitecharts.com` variant.
4. Enable HTTPS at the chosen host.

This repo already has an existing Pages deployment path for `ophiolite.dev`, so a second apex-domain Pages deployment from the same repository should not be treated as the final production plan.
