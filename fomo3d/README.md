# Fomo3D on Solana

This is a rebuild of the once popular game on Ethereum - Fomo3D. 

Tl;DR; it's a lottery-type game where users buy "keys" piling all proceeds in a single pot. Each time someone purchases a key, an internal timer bumps by 30s (up to a max of 24h). Then it starts counting down. The last user to have bought the key when the timer reaches zero takes home ~50% of the accumulated pot. The rest is split between other players ("f3d") / token holders ("p3d") / community rewards / affiliates and next round's pot.

There's a lot more detail to it, all of which is available on Fomo3D Wiki [here](https://fomo3d.hostedwiki.co/pages/Fomo3D%20Explained).

Original Ethereum code is available [here](https://gist.github.com/ilmoi/4daad0d6e9730cc6af833c065a95b717). The most important functions to look at are [buyCore](https://gist.github.com/ilmoi/4daad0d6e9730cc6af833c065a95b717#file-fomo-sol-L904), [reLoadCore](https://gist.github.com/ilmoi/4daad0d6e9730cc6af833c065a95b717#file-fomo-sol-L958) and [core](https://gist.github.com/ilmoi/4daad0d6e9730cc6af833c065a95b717#file-fomo-sol-L1009) + all the functions that they call. The rest is Ethereum-specific boilterplate or helper functions.   

This implementation follows the Wiki and the Ethereum implementation as closely as possible where it does make sense, and deviates where it doesn't. Eg the calculations around f3d share splitting felt unnecessary complex (maybe due to Ethereum specifics?) so we simply ignored them and implemented our own. Similarly, the Solana implementation relies heavily on PDAs which don't exist in Ethereum.

Other notes on implementation:
- Any wrapped token can be used as collateral for the game, but once the game is initialized it can't change
- The game creator initializes the game passing parameters related to round timing - but they don't have any special rights afterwards. All subsequent rounds and deadlings happen autonomously and all instructions can be called by all players
- Up to `u64::MAX` versions of the game can be deployed. This is done to allow room for experimentation with timing parameters, as well as to be able to run integration tests with fresh accounts
- Selected unit tests are done in rust, but the bulk of testing in integration tests in js (`client` dir)
- Front-end is coming!

To deploy your own version follow the standard steps of:
```shell
cargo build-bpf
solana program deploy path/to/fomo3d.so
```

To play with the js interface, follow the instructions inside the `client` dir's readme.

Fun fact to end on: how did the winner actually win the game? Wouldn't there always be someone who'd bump the timer, preventing the game from ever finishing? Read about it [here](https://medium.com/coinmonks/how-the-winner-got-fomo3d-prize-a-detailed-explanation-b30a69b7813f#:~:text=SECBIT%20Labs%20first%20found%20that,and%20increasing%20the%20winning%20probability.).