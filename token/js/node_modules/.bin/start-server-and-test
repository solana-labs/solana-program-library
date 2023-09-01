#!/usr/bin/env node

const debug = require('debug')('start-server-and-test')

const startAndTest = require('..').startAndTest
const utils = require('../utils')

const namedArguments = utils.getNamedArguments(process.argv.slice(2))
debug('named arguments: %o', namedArguments)

const args = utils.crossArguments(process.argv.slice(2))
debug('parsing CLI arguments: %o', args)
const parsed = utils.getArguments(args)
debug('parsed args: %o', parsed)

const { services, test } = parsed
if (!Array.isArray(services)) {
  throw new Error(`Could not parse arguments %o, got %o`, args, parsed)
}

if (!namedArguments.expect) {
  namedArguments.expect = 200
}

utils.printArguments({ services, test, namedArguments })

startAndTest({ services, test, namedArguments }).catch(e => {
  console.error(e)
  process.exit(1)
})
