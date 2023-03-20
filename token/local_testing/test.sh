#!/bin/sh
#!/bin/bash

cp ../../target/debug/spl-token .
cd local_testing
./spl-token create-account So11111111111111111111111111111111111111112 
./spl-token create-token
# MINT = 8iFyfyJSMe7PWndCsVbwUVAc28RVbhdfi8P5UN47K2oi
./spl-token create-account 8iFyfyJSMe7PWndCsVbwUVAc28RVbhdfi8P5UN47K2oi
# NATIVE_ATA = 9cHXbauQ8Wa3AVL6Ag38CoQhpT4sUortDqj4q7o7hKmd

# TOKEN ACCOUNT = FVMaoG1sZ2WquahmPEMeB6QsJai3pZryFNL8Ly4GwJ8W

# MULTISIG = 8D29tFTPoniQfP4SMuYJNQFE8N22KLMPWpvp8RiSzpqk

./spl-token create-multisig 2 wBWfn1WkVnUWbeW4U4iNy7xPmbs5F1c5SwRK7ABL1YL Du7k4wr15W5irBBNN1T1oymA5vbwBzKpxzJEMpqwb5bt AxE953xcx7XjD354DjRWDJV4ga2V63MNRWah4QMwEQhWDD
./spl-token create-account --fee-payer ../../../../club-program/wallets/club-wallet.json  --owner 8D29tFTPoniQfP4SMuYJNQFE8N22KLMPWpvp8RiSzpqk So11111111111111111111111111111111111111112
./spl-token recover-lamports 8D29tFTPoniQfP4SMuYJNQFE8N22KLMPWpvp8RiSzpqk --owner 8D29tFTPoniQfP4SMuYJNQFE8N22KLMPWpvp8RiSzpqk --multisig-signer ./signer1.json ./signer2.json ./signer3.json 