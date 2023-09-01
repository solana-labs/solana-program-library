# start-server-and-test

> Starts server, waits for URL, then runs test command; when the tests end, shuts down server

[![NPM][npm-icon] ][npm-url]

[![Build status][ci-image] ][ci-url]
[![semantic-release][semantic-image] ][semantic-url]
[![js-standard-style][standard-image]][standard-url]
[![renovate-app badge][renovate-badge]][renovate-app]

## Install

Requires [Node](https://nodejs.org/en/) version 8.9 or above.

```sh
npm install --save-dev start-server-and-test
```

## Upgrade

### v1 to v2

If you are using just the port number, and the resolved URL `localhost:xxxx` no longer works, use the explicit `http://localhost:xxxx` instead

```
# v1
$ npx start-test 3000
# v2
$ npx start-test http://localhost:3000
```

## Use

This command is meant to be used with NPM script commands. If you have a "start server", and "test" script names for example, you can start the server, wait for a url to respond, then run tests. When the test process exits, the server is shut down.

```json
{
    "scripts": {
        "start-server": "npm start",
        "test": "mocha e2e-spec.js",
        "ci": "start-server-and-test start-server http://localhost:8080 test"
    }
}
```

To execute all tests simply run `npm run ci`.

### Commands

In addition to using NPM script names, you can pass entire commands (surround them with quotes so it is still a single string) that will be executed "as is". For example, to start globally installed `http-server` before running and recording [Cypress.io](https://www.cypress.io) tests you can use

```shell
# run http-server, then when port 8000 responds run Cypress tests
start-server-and-test 'http-server -c-1 --silent' 8000 './node_modules/.bin/cypress run --record'
```

Because `npm` scripts execute with `./node_modules/.bin` in the `$PATH`, you can mix global and locally installed tools when using commands inside `package.json` file. For example, if you want to run a single spec file:

```json
{
  "scripts": {
    "ci": "start-server-and-test 'http-server -c-1 --silent' 8080 'cypress run --spec cypress/integration/location.spec.js'"
  }
}
```

Or you can move `http-server` part into its own `start` script, which is used by default and have the equivalent JSON

```json
{
  "scripts": {
    "start": "http-server -c-1 --silent",
    "ci": "start-server-and-test 8080 'cypress run --spec cypress/integration/location.spec.js'"
  }
}
```

Here is another example that uses Mocha

```json
{
  "scripts": {
    "ci": "start-server-and-test 'http-server -c-1 --silent' 8080 'mocha e2e-spec.js'"
  }
}
```

### Alias

You can use either `start-server-and-test`, `server-test` or `start-test` commands in your scripts.

You can use `:` in front of port number like `server-test :8080`, so all these are equivalent

```
start-server-and-test start http://127.0.0.1:8080 test
server-test start http://127.0.0.1:8080 test
server-test http://127.0.0.1:8080 test
server-test 127.0.0.1:8080 test
start-test :8080 test
start-test 8080 test
start-test 8080
```

**Tip:** I highly recommend you specify the full url instead of the port, see the `localhost vs 0.0.0.0 vs 127.0.0.1` section later in this README.

### Options

If you use convention and name your scripts "start" and "test" you can simply provide URL

```json
{
    "scripts": {
        "start": "npm start",
        "test": "mocha e2e-spec.js",
        "ci": "start-server-and-test http://localhost:8080"
    }
}
```

You can also shorten local url to just port, the code below is equivalent to checking `http://127.0.0.1:8080`.

```json
{
    "scripts": {
        "start": "npm start",
        "test": "mocha e2e-spec.js",
        "ci": "server-test 8080"
    }
}
```

You can provide first start command, port (or url) and implicit `test` command

```json
{
    "scripts": {
        "start-it": "npm start",
        "test": "mocha e2e-spec.js",
        "ci": "server-test start-it 8080"
    }
}
```

You can provide port number and custom test command, in that case `npm start` is assumed to start the server.

```json
{
    "scripts": {
        "start": "npm start",
        "test-it": "mocha e2e-spec.js",
        "ci": "server-test :9000 test-it"
    }
}
```

You can provide multiple resources to wait on, separated by a pipe `|`. _(be sure to wrap in quotes)_

```json
{
  "scripts": {
    "start": "npm start",
    "test-it": "mocha e2e-spec.js",
    "ci": "server-test \"8080|http://foo.com\""
  }
}
```

or for multiple ports simply: `server-test '8000|9000' test`.

If you want to start the server, wait for it to respond, and then run multiple test commands (and stop the server after they finish), you should be able to use `&&` to separate the test commands:

```json
{
  "scripts": {
    "start": "npm start",
    "test:unit": "mocha test.js",
    "test:e2e": "mocha e2e.js",
    "ci": "start-test 9000 'npm run test:unit && npm run test:e2e'"
  }
}
```

The above script `ci` after the `127.0.0.1:9000` responds executes the `npm run test:unit` command. Then when it finishes it runs `npm run test:e2e`. If the first or second command fails, the `ci` script fails. Of course, your mileage on Windows might vary.

#### expected

The server might respond, but require authorization, returning an error HTTP code by default. You can still know that the server is responding by using `--expect` argument (or its alias `--expected`):

```
$ start-test --expect 403 start :9000 test:e2e
```

See `demo-expect-403` NPM script.

Default expected value is 200.

## `npx` and `yarn`

If you have [npx](https://www.npmjs.com/package/npx) available, you can execute locally installed tools from the shell. For example, if the `package.json` has the following local tools:

```json
{
  "devDependencies": {
    "cypress": "3.2.0",
    "http-server": "0.11.1",
    "start-server-and-test": "1.9.0"
  }
}
```

Then you can execute tests simply:

```text
$ npx start-test 'http-server -c-1 .' 8080 'cypress run'
starting server using command "http-server -c-1 ."
and when url "http://127.0.0.1:8080" is responding
running tests using command "cypress run"
Starting up http-server, serving .
...
```

Similarly, you can use [yarn](https://yarnpkg.com/en/) to call locally installed tools

```text
$ yarn start-test 'http-server -c-1 .' 8080 'cypress run'
yarn run v1.13.0
$ /private/tmp/test-t/node_modules/.bin/start-test 'http-server -c-1 .' 8080 'cypress run'
starting server using command "http-server -c-1 ."
and when url "http://127.0.0.1:8080" is responding
running tests using command "cypress run"
Starting up http-server, serving .
...
```

## localhost vs 0.0.0.0 vs 127.0.0.1

The latest versions of Node and some web servers listen on host `0.0.0.0` which _no longer means localhost_. Thus if you specify _just the port number_, like `:3000`, this package will try `http://127.0.0.1:3000` to ping the server. A good practice is to specify the full URL you would like to ping.

```
# same as "http://127.0.0.1:3000"
start-server start 3000 test
# better
start-server start http://127.0.0.1:3000 test
# or
start-server start http://0.0.0.0:3000 test
# of course, if your server is listening on localhost
# you can still set the URL
start-server start http://localhost:3000 test
```

## Note for yarn users

By default, npm is used to run scripts, however you can specify that yarn is used as follows:

```json
"scripts": {
    "start-server": "yarn start",
    "test": "mocha e2e-spec.js",
    "ci": "start-server-and-test 'yarn start-server' http://localhost:8080 'yarn test'"
}
```

## Note for webpack-dev-server users

Also applies to **Vite** users!

If you are using [webpack-dev-server](https://www.npmjs.com/package/webpack-dev-server) (directly or via `angular/cli` or other boilerplates) then the server does not respond to HEAD requests from `start-server-and-test`. You can check if the server responds to the HEAD requests by starting the server and pinging it from another terminal using `curl`

```
# from the first terminal start the server
$ npm start
# from the second terminal call the server with HEAD request
$ curl --head http://localhost:3000
```

If the server responds with 404, then it does not handle the HEAD requests. You have two solutions:

### Use HTTP GET requests

You can force the `start-server-and-test` to ping the server using GET requests using the `http-get://` prefix:


```
start-server-and-test http-get://localhost:8080
```

### Ping a specific resource

As an alternative to using GET method to request the root page, you can try pinging a specific resource, see the discussion in the [issue #4](https://github.com/bahmutov/start-server-and-test/issues/4).

```
# maybe the server responds to HEAD requests to the HTML page
start-server-and-test http://localhost:3000/index.html
# or maybe the server responds to HEAD requests to JS resource
start-server-and-test http://localhost:8080/app.js
```

### Explanation

You can watch the explanation in the video [Debug a Problem in start-server-and-test](https://youtu.be/rxyZOxYCsAk).

Under the hood this module uses [wait-on](https://github.com/jeffbski/wait-on) to ping the server. Wait-on uses `HEAD` by default, but `webpack-dev-server` does not respond to `HEAD` only to `GET` requests. Thus you need to use `http-get://` URL format to force `wait-on` to use `GET` probe or ask for a particular resource.

### Debugging

To see diagnostic messages, run with environment variable `DEBUG=start-server-and-test`

```
$ DEBUG=start-server-and-test npm run test
  start-server-and-test parsing CLI arguments: [ 'dev', '3000', 'subtask' ] +0ms
  start-server-and-test parsed args: { services: [ { start: 'npm run dev', url: [Array] } ], test: 'npm run subtask' }
...
making HTTP(S) head request to url:http://127.0.0.1:3000 ...
  HTTP(S) error for http://127.0.0.1:3000 Error: Request failed with status code 404
```

### Disable HTTPS certificate checks

To disable HTTPS checks for `wait-on`, run with environment variable `START_SERVER_AND_TEST_INSECURE=1`.

### Timeout

This utility will wait for maximum of 5 minutes while checking for the server to respond (default). Setting an environment variable `WAIT_ON_TIMEOUT=600000` (milliseconds) sets the timeout for example to 10 minutes.

### Interval

This utility will check for a server response every two seconds (default). Setting an environment variable `WAIT_ON_INTERVAL=600000` (milliseconds) sets the interval for example to 10 minutes.

### Starting two servers

Sometimes you need to start one API server and one webserver in order to test the application. Use the syntax:

```
start-test <first command> <first resource> <second command> <second resource> <test command>
```

For example if API runs at port 3000 and server runs at port 8080:

```json
{
  "scripts": {
    "test": "node src/test",
    "start:api": "node src/api",
    "start:server": "node src/server",
    "test:all": "start-test start:api 3000 start:server 8080 test"
  }
}
```

In the above example you would run `npm run test:all` to start the API first, then when it responds, start the server, and when the server is responding, it would run the tests. After the tests finish, it will shut down both servers. See the repo [start-two-servers-example](https://github.com/bahmutov/start-two-servers-example) for full example

## Note for Apollo Server users

When passing a simple GET request to Apollo Server it will respond with a 405 error. To get around this problem you need to pass a valid GraphQL query into the query parameter. Passing in a basic schema introspection query will work to determine the presence of an Apollo Server. You can configure your npm script like so:

```json
{
  "scripts": {
    "ci": "start-server-and-test start 'http-get://localhost:4000/graphql?query={ __schema { queryType { name } } }' test"
  }
}
```

### Small print

Author: Gleb Bahmutov &lt;gleb.bahmutov@gmail.com&gt; &copy; 2017

* [@bahmutov](https://twitter.com/bahmutov)
* [glebbahmutov.com](https://glebbahmutov.com)
* [blog](https://glebbahmutov.com/blog)

License: MIT - do anything with the code, but don't blame me if it does not work.

Support: if you find any problems with this module, email / tweet /
[open issue](https://github.com/bahmutov/start-server-and-test/issues) on Github

## MIT License

See [LICENSE](./LICENSE)

[npm-icon]: https://nodei.co/npm/start-server-and-test.svg?downloads=true
[npm-url]: https://npmjs.org/package/start-server-and-test
[ci-image]: https://github.com/bahmutov/start-server-and-test/workflows/ci/badge.svg?branch=master
[ci-url]: https://github.com/bahmutov/start-server-and-test/actions
[semantic-image]: https://img.shields.io/badge/%20%20%F0%9F%93%A6%F0%9F%9A%80-semantic--release-e10079.svg
[semantic-url]: https://github.com/semantic-release/semantic-release
[standard-image]: https://img.shields.io/badge/code%20style-standard-brightgreen.svg
[standard-url]: http://standardjs.com/
[renovate-badge]: https://img.shields.io/badge/renovate-app-blue.svg
[renovate-app]: https://renovateapp.com/
