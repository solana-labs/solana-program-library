# Docs Readme

SPL Docs are built using [Docusaurus 2](https://v2.docusaurus.io/) with `npm`.
Static content delivery is handled using `vercel`.

### Installing Docusaurus

```
$ npm install
```

### Local Development

This command starts a local development server and open up a browser window.
Most changes are reflected live without having to restart the server.

```
$ npm run start
```

### Build Locally

This command generates static content into the `build` directory and can be
served using any static contents hosting service.

```
$ docs/build.sh
```

### CI Build Flow

The docs are built and published in Travis CI with the `docs/build.sh` script.
On each PR, the docs are built, but not published.

In each post-commit build, docs are built and published using `vercel`.

Documentation is published to spl.solana.com
