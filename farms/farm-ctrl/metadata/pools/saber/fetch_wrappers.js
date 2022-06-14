const anchor = require("@project-serum/anchor");

anchor.setProvider(anchor.Provider.env());

async function fetch_wrapper_vault(symbol, wrapper_program, wrapper) {
  const idl = JSON.parse(
    require("fs").readFileSync("./add_decimals_idl.json", "utf8")
  );
  const programId = new anchor.web3.PublicKey(wrapper_program);
  const program = new anchor.Program(idl, programId);
  console.log(
    symbol,
    (
      await program.account.wrappedToken.fetch(wrapper)
    ).wrapperUnderlyingTokens.toString()
  );
}

console.log("Running client.");
fetch_wrapper_vault(
  "swhETH-9",
  "DecZY86MU5Gj7kppfUCEmd4LbXXuyZH1yHaP2NTqdiZB",
  "93qsLbASEG8DmtSB2MEVaa25KvEm2afh5rzbaAJHLi5A"
).then(() => console.log("Success"));

fetch_wrapper_vault(
  "swFTT-9",
  "DecZY86MU5Gj7kppfUCEmd4LbXXuyZH1yHaP2NTqdiZB",
  "FCgoT8RpsopdM5QT6AB98NUfUnDnu7y865MFpRx93JrS"
).then(() => console.log("Success"));

fetch_wrapper_vault(
  "srenBTC-10",
  "DecZY86MU5Gj7kppfUCEmd4LbXXuyZH1yHaP2NTqdiZB",
  "3A85wiQg2REhBVxVS1CjDaS333TBNM2g37BbdNGSMheg"
).then(() => console.log("Success"));

fetch_wrapper_vault(
  "srenBTC-9",
  "DecZY86MU5Gj7kppfUCEmd4LbXXuyZH1yHaP2NTqdiZB",
  "D231Uoh24bXtUtWN51ZbFAFSBmGT3zuuEAHZNuCmtRjN"
).then(() => console.log("Success"));

fetch_wrapper_vault(
  "srenLUNA-9",
  "DecZY86MU5Gj7kppfUCEmd4LbXXuyZH1yHaP2NTqdiZB",
  "FDGtFWVhEb1zxnaW2FzogeGDxLoAV7Cu9XdNYPEVwqt"
).then(() => console.log("Success"));

fetch_wrapper_vault(
  "sUSDC-8",
  "DecZY86MU5Gj7kppfUCEmd4LbXXuyZH1yHaP2NTqdiZB",
  "G4gRGymKo7MGzGZup12JS39YVCvy8YMM6KY9AmcKi5iw"
).then(() => console.log("Success"));

fetch_wrapper_vault(
  "sUSDC-9",
  "DecZY86MU5Gj7kppfUCEmd4LbXXuyZH1yHaP2NTqdiZB",
  "AnKLLfpMcceM6YXtJ9nGxYekVXqfWy8WNsMZXoQTCVQk"
).then(() => console.log("Success"));

fetch_wrapper_vault(
  "sUSDT-9",
  "DecZY86MU5Gj7kppfUCEmd4LbXXuyZH1yHaP2NTqdiZB",
  "F9TsAsh5RirU3LqyTJECLQEGXnF4RQT7ckvexCP1KNTu"
).then(() => console.log("Success"));

fetch_wrapper_vault(
  "sBTC-8",
  "DecZY86MU5Gj7kppfUCEmd4LbXXuyZH1yHaP2NTqdiZB",
  "GpkFF2nPfjUcsavgDGscxaUEQ2hYJ563AXXtU8ohiZ7c"
).then(() => console.log("Success"));

fetch_wrapper_vault(
  "sBTC-9",
  "DecZY86MU5Gj7kppfUCEmd4LbXXuyZH1yHaP2NTqdiZB",
  "7hWjnVC6FNkmmgjq88LEnRycrKvxVB1MsJ6FQcrvxe4n"
).then(() => console.log("Success"));

fetch_wrapper_vault(
  "sETH-8",
  "DecZY86MU5Gj7kppfUCEmd4LbXXuyZH1yHaP2NTqdiZB",
  "fvSvtHNFuDHrAN82YEyBApRs3U6vUGCLzKGMuPmCaF8"
).then(() => console.log("Success"));

fetch_wrapper_vault(
  "sFTT-9",
  "DecZY86MU5Gj7kppfUCEmd4LbXXuyZH1yHaP2NTqdiZB",
  "2ffwMLE4dxSv59eYXhfhfuS81kz6gzf6DZjdBxRHZz9A"
).then(() => console.log("Success"));

fetch_wrapper_vault(
  "ssoFTT-8",
  "DecZY86MU5Gj7kppfUCEmd4LbXXuyZH1yHaP2NTqdiZB",
  "CGxMr5UrTjApBjU656N9NBAsGby4fWs1KgVtueQ8WKt6"
).then(() => console.log("Success"));

fetch_wrapper_vault(
  "sETH-9",
  "DecZY86MU5Gj7kppfUCEmd4LbXXuyZH1yHaP2NTqdiZB",
  "93qsLbASEG8DmtSB2MEVaa25KvEm2afh5rzbaAJHLi5A"
).then(() => console.log("Success"));

fetch_wrapper_vault(
  "sagEUR-9",
  "DecZY86MU5Gj7kppfUCEmd4LbXXuyZH1yHaP2NTqdiZB",
  "EhQqUmkUXXnxmV7yA6PDrQWvLgSd9HkrwdDKk1B5m6Tc"
).then(() => console.log("Success"));

fetch_wrapper_vault(
  "sCASH-8",
  "DecZY86MU5Gj7kppfUCEmd4LbXXuyZH1yHaP2NTqdiZB",
  "Ffxi5TSpFV9NeV5KyNDCC7fWnFoFd2bDcL1eViSAE2M2"
).then(() => console.log("Success"));

fetch_wrapper_vault(
  "sCASH-9",
  "DecZY86MU5Gj7kppfUCEmd4LbXXuyZH1yHaP2NTqdiZB",
  "2B5Qedoo95Pjpv9xVPw82bbmcGDGCNHroKpzQE2CNHRZ"
).then(() => console.log("Success"));

fetch_wrapper_vault(
  "sLUNA-9",
  "DecZY86MU5Gj7kppfUCEmd4LbXXuyZH1yHaP2NTqdiZB",
  "ACvLVgR3UKdDB3b1QapsbJsPXaUrBPdJGDfiFnMYMXoz"
).then(() => console.log("Success"));

fetch_wrapper_vault(
  "sUST-8",
  "DecZY86MU5Gj7kppfUCEmd4LbXXuyZH1yHaP2NTqdiZB",
  "EwWpia5t9Twiwdi8ghK8e8JHaf6ShNU9jmoYpvdZhBwC"
).then(() => console.log("Success"));

fetch_wrapper_vault(
  "sUST-9",
  "DecZY86MU5Gj7kppfUCEmd4LbXXuyZH1yHaP2NTqdiZB",
  "FPuYMuodknZuQKHA8Wp4PBbp52Qu8nK2oAuwedp2WfM3"
).then(() => console.log("Success"));

fetch_wrapper_vault(
  "ssoFTT-9",
  "DecZY86MU5Gj7kppfUCEmd4LbXXuyZH1yHaP2NTqdiZB",
  "2ffwMLE4dxSv59eYXhfhfuS81kz6gzf6DZjdBxRHZz9A"
).then(() => console.log("Success"));

fetch_wrapper_vault(
  "sUSDT-8",
  "DecZY86MU5Gj7kppfUCEmd4LbXXuyZH1yHaP2NTqdiZB",
  "GiLSv94Wwyd6suH57Fu6HjEKsMxhNGfEwKn9vT22me1p"
).then(() => console.log("Success"));

fetch_wrapper_vault(
  "ssoETH-8",
  "DecZY86MU5Gj7kppfUCEmd4LbXXuyZH1yHaP2NTqdiZB",
  "fvSvtHNFuDHrAN82YEyBApRs3U6vUGCLzKGMuPmCaF8"
).then(() => console.log("Success"));
