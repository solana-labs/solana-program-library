#!/usr/bin/env node

const ghpages = require('../lib/index.js');

function main() {
  ghpages.clean();
}

if (require.main === module) {
  main();
}

module.exports = main;
