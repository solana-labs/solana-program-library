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