# `morpheos.net/voice` site hand-off

## Artefact

The publishable static source is `website/`:

- `index.html` — product/download page;
- `models.html` — local model guidance;
- `contact.html` — GitHub/email support routes;
- `styles.css` and `app.js` — local presentation/interaction;
- `favicon.svg`, `morpheos-voice-mark.svg`, `morpheos-voice-lockup.svg` — local brand assets;
- `USER_GUIDE.md` and `LAUNCH_GUIDE.md` — linked documentation/maintainer guidance.

It uses relative local assets and canonical metadata for `https://morpheos.net/voice`. It has no analytics, remote font, contact-form processor or lead-capture dependency.

## Product role

Voice is the free product attractor for MorpheOS. The page leads with the product, open-source trust, privacy boundary and downloads. One quiet route back to `morpheos.net` appears after the core journey. Do not place pricing, an account gate or a paid CTA ahead of the free download.

## Deployment integration

1. Mount or copy the contents of `website/` at the parent site's `/voice` route.
2. Preserve relative asset resolution for `/voice/index.html`, `/voice/models.html` and `/voice/contact.html`.
3. If the parent router removes `.html`, add tested redirects rather than rewriting links by assumption.
4. Reuse the parent IBM Plex fonts when already loaded; do not add a new remote font provider.
5. Keep the page's MorpheOS palette tokens aligned with the parent site.
6. Do not use the legacy `deploy/nginx/oswispa.tylerbuilds.com.conf`; it remains historical infrastructure.

## Publication gate

Do not deploy this hand-off until:

- the approved GitHub Release exists;
- every platform button resolves to an artefact in that release;
- signing/platform limitations match the release notes;
- `SHA256SUMS` covers the complete download set;
- public-download smoke passes;
- `python3 scripts/check_website.py` passes against the release version;
- desktop and 390 px browser checks show no overflow/console errors; and
- Tyler approves the production change.

After deployment, verify the canonical URL, metadata, favicon, local pages, every download and the parent MorpheOS link from an external network. Record the deployment commit and response checks in `05_RELEASE_READINESS.md`.
