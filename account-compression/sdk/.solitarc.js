// @ts-check
const path = require('path');
const programDir = path.join(__dirname, '..', 'programs', 'account-compression');
const idlDir = path.join(__dirname, 'idl');
const sdkDir = path.join(__dirname, 'src', 'generated');
const binaryInstallDir = path.join(__dirname, '..', 'target', 'solita');

module.exports = {
  idlGenerator: 'anchor',
  programName: 'spl_account_compression',
  programId: 'GRoLLzvxpxxu2PGNJMMeZPyMxjAUH9pKqxGXV9DGiceU',
  idlDir,
  sdkDir,
  binaryInstallDir,
  programDir,
};
