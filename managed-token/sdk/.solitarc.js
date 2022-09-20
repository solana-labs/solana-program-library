const path = require("path");
const programDir = path.join(__dirname, "..", "program");
const idlDir = path.join(__dirname, "idl");
const sdkDir = path.join(__dirname, "src", "generated");
const binaryInstallDir = path.join(__dirname, "..", "..", "target", "solita");

module.exports = {
  idlGenerator: "shank",
  programName: "spl_managed_token",
  idlDir,
  sdkDir,
  binaryInstallDir,
  programDir,
};
