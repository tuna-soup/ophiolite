# lithos-docs

Public documentation site source for `lithos.dev`.

## Local Development

```powershell
bun install
bun run dev
```

## Build

```powershell
bun run build
```

## Deployment

The site is deployed with GitHub Pages from `.github/workflows/docs-site.yml`.

The intended production host is:

- `https://lithos.dev`

Required domain steps outside the repo:

1. Buy `lithos.dev`.
2. Configure GitHub Pages custom domain to `lithos.dev`.
3. Add the required apex DNS records for GitHub Pages.
4. Add `www` as a `CNAME` if you want a `www.lithos.dev` redirect.
5. Enable HTTPS in GitHub Pages.
