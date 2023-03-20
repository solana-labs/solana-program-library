#!/bin/sh
#!/bin/bash

cp ../../target/release/spl-token .
cd local_testing
# NATIVE_ATA = 9cHXbauQ8Wa3AVL6Ag38CoQhpT4sUortDqj4q7o7hKmd
./spl-token create-account So11111111111111111111111111111111111111112 

# TOKEN ACCOUNT = FVMaoG1sZ2WquahmPEMeB6QsJai3pZryFNL8Ly4GwJ8W

# MULTISIG = 8D29tFTPoniQfP4SMuYJNQFE8N22KLMPWpvp8RiSzpqk

./spl-token create-multisig 2 wBWfn1WkVnUWbeW4U4iNy7xPmbs5F1c5SwRK7ABL1YL Du7k4wr15W5irBBNN1T1oymA5vbwBzKpxzJEMpqwb5bt AxE953xcx7XjD354DjRWDJV4ga2V63MNRWah4QMwEQhWDD
./spl-token create-account --fee-payer ../../../../club-program/wallets/club-wallet.json  --owner 8D29tFTPoniQfP4SMuYJNQFE8N22KLMPWpvp8RiSzpqk So11111111111111111111111111111111111111112
./spl-token recover-lamports 8D29tFTPoniQfP4SMuYJNQFE8N22KLMPWpvp8RiSzpqk --owner 8D29tFTPoniQfP4SMuYJNQFE8N22KLMPWpvp8RiSzpqk --multisig-signer ./signer1.json ./signer2.json ./signer3.json 
./spl-token create-token
# MINT = DKQ1eyRK8PLTLR9ov4MrxCoUpi2BSY2m8WxG3D2TzvHY
./spl-token create-account GD9NydjePheJpHjZSPMHoh46aLQevWjWFQ1eCS6FgwCG

# TOKEN ACCOUNT = FhrnqtRBXaVHsFqcCfS3XDf8M6Cmz5FzLVgzAX7wo5PS

# MULTISIG = DJ17Xjf7Buhw4MCXFNrufxkjLej2HgEqeVggheqfo6ZC
