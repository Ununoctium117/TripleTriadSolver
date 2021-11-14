# Triple Triad Solver

This is a Triple Triad solver for FFXIV.

## How to Use

* Either build the solver yourself (see below), or download the latest release.
* Because the actual Triple Triad cards themselves (and the NPC data) is owned by Square Enix, it isn't redistributed with this program. Therefore, it will ask you to enter the location of a Github repository that does host the data, and then download and cache it.

## Limitations:

* Plus and Same aren't handled yet (no reason they can't, I just haven't implemented it yet).
* Swap isn't handled yet (and handling it would require more work than Plus/Same).
* Chaos isn't handled yet (ie, telling you the best place to play your one card).
* Some NPCs seem to be missing - why?
* Regional rules aren't detected, only NPC-specific rules.
* No way to clear and refresh the data cache (to work around this, you can delete it manually from %LOCALAPPDATA%\Ununoctium\TripleTriadSolver\cache and restart the program).
* There should be nothing platform specific, so it should work on Windows, Mac, and Linux, but I've only tested with Windows.

## Technical stuff:

This predicts the best move using Negamax search with Alpha-Beta pruning. This is fast enough to explore the entire game tree.

It's common for decks to be so much better than others that with perfect play, one player will always win. This causes all moves to have equal value, which isn't the most useful outcome. Therefore, there's an additional Monte-Carlo simulation to break the ties.

## Building:

* Install the Rust compiler and package manager: https://www.rust-lang.org/tools/install
* Clone this repository and navigate to where you did so on the command line.
* Build with `cargo build --release`; this will download and compile all dependencies and generate `TripleTriadSolver.exe` in `target/release`.
