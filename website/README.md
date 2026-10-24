# pmux-explorer website

Documentation site for [pmux-explorer](https://github.com/slatkin/pmux-explorer), built with [Astro Starlight](https://starlight.astro.build/).

## Local development

```sh
npm install
npm run dev
```

Open <http://localhost:4321/pmux-explorer/> to view.

## Build

```sh
npm run build     # generates dist/
npm run preview   # serves built site locally
```

Deployed to GitHub Pages by `.github/workflows/deploy-website.yml` on every push to `main` that touches `website/**` or the workflow file itself.
