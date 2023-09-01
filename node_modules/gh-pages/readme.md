
# gh-pages

Publish files to a `gh-pages` branch on GitHub (or any other branch anywhere else).

## Getting Started

```shell
npm install gh-pages --save-dev
```

This module requires Git >= 1.9 and Node >= 12.

## Basic Usage

```js
var ghpages = require('gh-pages');

ghpages.publish('dist', function(err) {});
```


## `publish`

```js
ghpages.publish(dir, callback);
// or...
ghpages.publish(dir, options, callback);
```

Calling this function will create a temporary clone of the current repository, create a `gh-pages` branch if one doesn't already exist, copy over all files from the base path, or only those that match patterns from the optional `src` configuration, commit all changes, and push to the `origin` remote.

If a `gh-pages` branch already exists, it will be updated with all commits from the remote before adding any commits from the provided `src` files.

**Note** that any files in the `gh-pages` branch that are *not* in the `src` files **will be removed**.  See the [`add` option](#optionsadd) if you don't want any of the existing files removed.


### <a id="dir">`dir`</a>
* type: `string`

The base directory for all source files (those listed in the `src` config property).

Example use:

```js
/**
 * Given the following directory structure:
 *
 *   dist/
 *     index.html
 *     js/
 *       site.js
 *
 * The usage below will create a `gh-pages` branch that looks like this:
 *
 *   index.html
 *   js/
 *     site.js
 *
 */
ghpages.publish('dist', callback);
```


### Options

The default options work for simple cases.  The options described below let you push to alternate branches, customize your commit messages, and more.


#### <a id="optionssrc">options.src</a>
 * type: `string|Array<string>`
 * default: `'**/*'`

The [minimatch](https://github.com/isaacs/minimatch) pattern or array of patterns used to select which files should be published.


#### <a id="optionsbranch">options.branch</a>
 * type: `string`
 * default: `'gh-pages'`
 * `-b | --branch <branch name>`

The name of the branch you'll be pushing to.  The default uses GitHub's `gh-pages` branch, but this can be configured to push to any branch on any remote.

Example use of the `branch` option:

```js
/**
 * This task pushes to the `master` branch of the configured `repo`.
 */
ghpages.publish('dist', {
  branch: 'master',
  repo: 'https://example.com/other/repo.git'
}, callback);
```


#### <a id="optionsdest">options.dest</a>
 * type: `string`
 * default: `'.'`

The destination folder within the destination branch.  By default, all files are published to the root of the repository.

Example use of the `dest` option:

```js
/**
 * Place content in the static/project subdirectory of the target
 * branch.
 */
ghpages.publish('dist', {
  dest: 'static/project'
}, callback);
```

#### <a id="optionsdotfiles">options.dotfiles</a>
 * type: `boolean`
 * default: `false`

Include dotfiles.  By default, files starting with `.` are ignored unless they are explicitly provided in the `src` array.  If you want to also include dotfiles that otherwise match your `src` patterns, set `dotfiles: true` in your options.

Example use of the `dotfiles` option:

```js
/**
 * The usage below will push dotfiles (directories and files)
 * that otherwise match the `src` pattern.
 */
ghpages.publish('dist', {dotfiles: true}, callback);
```


#### <a id="optionsadd">options.add</a>
 * type: `boolean`
 * default: `false`

Only add, and never remove existing files.  By default, existing files in the target branch are removed before adding the ones from your `src` config.  If you want the task to add new `src` files but leave existing ones untouched, set `add: true` in your options.

Example use of the `add` option:

```js
/**
 * The usage below will only add files to the `gh-pages` branch, never removing
 * any existing files (even if they don't exist in the `src` config).
 */
ghpages.publish('dist', {add: true}, callback);
```


#### <a id="optionsrepo">options.repo</a>
 * type: `string`
 * default: url for the origin remote of the current dir (assumes a git repository)
 * `-r | --repo <repo url>`

By default, `gh-pages` assumes that the current working directory is a git repository, and that you want to push changes to the `origin` remote.

If instead your script is not in a git repository, or if you want to push to another repository, you can provide the repository URL in the `repo` option.

Example use of the `repo` option:

```js
/**
 * If the current directory is not a clone of the repository you want to work
 * with, set the URL for the repository in the `repo` option.  This usage will
 * push all files in the `src` config to the `gh-pages` branch of the `repo`.
 */
ghpages.publish('dist', {
  repo: 'https://example.com/other/repo.git'
}, callback);
```


#### <a id="optionsremote">options.remote</a>
 * type: `string`
 * default: `'origin'`

The name of the remote you'll be pushing to.  The default is your `'origin'` remote, but this can be configured to push to any remote.

Example use of the `remote` option:

```js
/**
 * This task pushes to the `gh-pages` branch of of your `upstream` remote.
 */
ghpages.publish('dist', {
  remote: 'upstream'
}, callback);
```


#### <a id="optionstag">options.tag</a>
 * type: `string`
 * default: `''`

Create a tag after committing changes on the target branch.  By default, no tag is created.  To create a tag, provide the tag name as the option value.


#### <a id="optionsmessage">options.message</a>
 * type: `string`
 * default: `'Updates'`

The commit message for all commits.

Example use of the `message` option:

```js
/**
 * This adds commits with a custom message.
 */
ghpages.publish('dist', {
  message: 'Auto-generated commit'
}, callback);
```


#### <a id="optionsuser">options.user</a>
 * type: `Object`
 * default: `null`

If you are running the `gh-pages` task in a repository without a `user.name` or `user.email` git config properties (or on a machine without these global config properties), you must provide user info before git allows you to commit.  The `options.user` object accepts `name` and `email` string values to identify the committer.

Example use of the `user` option:

```js
ghpages.publish('dist', {
  user: {
    name: 'Joe Code',
    email: 'coder@example.com'
  }
}, callback);
```

#### <a id="optionsuser">options.remove</a>
 * type: `string`
 * default: `'.'`

Removes files that match the given pattern (Ignored if used together with
`--add`). By default, `gh-pages` removes everything inside the target branch
auto-generated directory before copying the new files from `dir`.

Example use of the `remove` option:

```js
ghpages.publish('dist', {
  remove: "*.json"
}, callback);
```


#### <a id="optionspush">options.push</a>
 * type: `boolean`
 * default: `true`

Push branch to remote.  To commit only (with no push) set to `false`.

Example use of the `push` option:

```js
ghpages.publish('dist', {push: false}, callback);
```


#### <a id="optionshistory">options.history</a>
 * type: `boolean`
 * default: `true`

Push force new commit without parent history.

Example use of the `history` option:

```js
ghpages.publish('dist', {history: false}, callback);
```


#### <a id="optionssilent">options.silent</a>
 * type: `boolean`
 * default: `false`

Avoid showing repository URLs or other information in errors.

Example use of the `silent` option:

```js
/**
 * This configuration will avoid logging the GH_TOKEN if there is an error.
 */
ghpages.publish('dist', {
  repo: 'https://' + process.env.GH_TOKEN + '@github.com/user/private-repo.git',
  silent: true
}, callback);
```


#### <a id="optionsbeforeadd">options.beforeAdd</a>
 * type: `function`
 * default: `null`

Custom callback that is executed right before `git add`.

The CLI expects a file exporting the beforeAdd function

```bash
gh-pages --before-add ./cleanup.js
```

Example use of the `beforeAdd` option:

```js
/**
 * beforeAdd makes most sense when `add` option is active
 * Assuming we want to keep everything on the gh-pages branch
 * but remove just `some-outdated-file.txt`
 */
ghpages.publish('dist', {
  add: true,
  async beforeAdd(git) {
    return git.rm('./some-outdated-file.txt');
  }
}, callback);
```


#### <a id="optionsgit">options.git</a>
 * type: `string`
 * default: `'git'`

Your `git` executable.

Example use of the `git` option:

```js
/**
 * If `git` is not on your path, provide the path as shown below.
 */
ghpages.publish('dist', {
  git: '/path/to/git'
}, callback);
```

## Command Line Utility

Installing the package creates a `gh-pages` command line utility.  Run `gh-pages --help` to see a list of supported options.

With a local install of `gh-pages`, you can set up a package script with something like the following:

```shell
"scripts": {
  "deploy": "gh-pages -d dist"
}
```

And then to publish everything from your `dist` folder to your `gh-pages` branch, you'd run this:

```shell
npm run deploy
```

## GitHub Pages Project Sites

There are three types of GitHub Pages sites: [project, user, and organization](https://docs.github.com/en/pages/getting-started-with-github-pages/about-github-pages#types-of-github-pages-sites). Since project sites are not hosted on the root `<user|org>.github.io` domain and instead under a URL path based on the repository name, they often require configuration tweaks for various build tools and frameworks. If not configured properly, a browser will usually log `net::ERR_ABORTED 404` errors when looking for compiled assets.

Examples:
- Create React App (which uses webpack under the hood) [requires the user to set a `"homepage"` property in their `package.json` so that built assets are referenced correctly in the final compiled HTML](https://create-react-app.dev/docs/deployment/#building-for-relative-paths).
  - This [has been often been thought of as an issue with `gh-pages`](https://github.com/tschaub/gh-pages/issues/285#issuecomment-805321474), though this package isn't able to control a project's build configuration.
- Vite [requires a `"base"` property in its `vite.config.js`](https://vitejs.dev/guide/static-deploy.html#github-pages)
- Next.js does not support deploying to GitHub Pages [because of an opinionated static export approach that puts all assets under a `_next` direcotry that GitHub Pages ignores](https://github.com/vercel/next.js/issues/9460).

When using a project site, be sure to read the documentation for your particular build tool or framework to learn how to configure correct asset paths.

## Debugging

To get additional output from the `gh-pages` script, set `NODE_DEBUG=gh-pages`.  For example:

```shell
NODE_DEBUG=gh-pages npm run deploy
```

## Dependencies

Note that this plugin requires Git 1.9 or higher (because it uses the `--exit-code` option for `git ls-remote`).  If you'd like to see this working with earlier versions of Git, please [open an issue](https://github.com/tschaub/gh-pages/issues).

![Test Status](https://github.com/tschaub/gh-pages/workflows/Test/badge.svg)

## Tips

### when get error `branch already exists`
```
{ ProcessError: fatal: A branch named 'gh-pages' already exists.

    at ChildProcess.<anonymous> (~/node_modules/gh-pages/lib/git.js:42:16)
    at ChildProcess.emit (events.js:180:13)
    at maybeClose (internal/child_process.js:936:16)
    at Process.ChildProcess._handle.onexit (internal/child_process.js:220:5)
  code: 128,
  message: 'fatal: A branch named \'gh-pages\' already exists.\n',
  name: 'ProcessError' }
  ```

The `gh-pages` module writes temporary files to a `node_modules/.cache/gh-pages` directory.  The location of this directory can be customized by setting the `CACHE_DIR` environment variable.

If `gh-pages` fails, you may find that you need to manually clean up the cache directory.  To remove the cache directory, run `node_modules/gh-pages/bin/gh-pages-clean` or remove `node_modules/.cache/gh-pages`.

### Deploying to github pages with custom domain

Modify the deployment line to your deploy script if you use custom domain. This will prevent the deployment from removing the domain settings in GitHub.

```
echo your_cutom_domain.online > ./build/CNAME && gh-pages -d build"
```

### Deploying with GitHub Actions

In order to deploy with GitHub Actions, you will need to define a user and set the git repository for the process. See the example step below

```yaml
- name: Deploy with gh-pages
  run: |
    git remote set-url origin https://git:${GITHUB_TOKEN}@github.com/${GITHUB_REPOSITORY}.git
    npx gh-pages -d build -u "github-actions-bot <support+actions@github.com>"
   env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

The `secrets.GITHUB_TOKEN` is provided automatically as part of the GitHub Action and does not require any further configuration, but simply needs to be passed in as an environmental variable to the step. `GITHUB_REPOSITORY` is the owner and repository name and is also passed in automatically, but does not need to be added to the `env` list.

See [Issue #345](https://github.com/tschaub/gh-pages/issues/345) for more information

#### Deploying with GitHub Actions and a named script

If you are using a named script in the `package.json` file to deploy, you will need to ensure you pass the variables properly to the wrapped `gh-pages` script. Given the `package.json` script below:

```json
"scripts": {
  "deploy": "gh-pages -d build"
}
```

You will need to utilize the `--` option to pass any additional arguments:

```yaml
- name: Deploy with gh-pages
  run: |
    git remote set-url origin https://git:${GITHUB_TOKEN}@github.com/${GITHUB_REPOSITORY}.git
    npm run deploy -- -u "github-actions-bot <support+actions@github.com>"
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

See [Pull Request #368](https://github.com/tschaub/gh-pages/pull/368) for more information.
