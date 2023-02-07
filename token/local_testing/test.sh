#!/bin/sh
#!/bin/bash

cp ../../target/release/spl-token .
cd local_testing
# NATIVE_ATA = 9cHXbauQ8Wa3AVL6Ag38CoQhpT4sUortDqj4q7o7hKmd
./spl-token create-account So11111111111111111111111111111111111111112 

./spl-token create-token
# MINT = DKQ1eyRK8PLTLR9ov4MrxCoUpi2BSY2m8WxG3D2TzvHY
./spl-token create-account GD9NydjePheJpHjZSPMHoh46aLQevWjWFQ1eCS6FgwCG

# TOKEN ACCOUNT = FhrnqtRBXaVHsFqcCfS3XDf8M6Cmz5FzLVgzAX7wo5PS

# MULTISIG = DJ17Xjf7Buhw4MCXFNrufxkjLej2HgEqeVggheqfo6ZC