# Tuxphones

Discord screensharing audio for Linux.

Development is ongoing and therefore some features do not work fully or are broken. If you find a bug, open an issue!

## Installation
### Prerequisites
- BetterDiscord
- Rust
- Cargo
- Systemd
- PulseAudio Dev Libraries
- All GStreamer Dev Libraries

### Tuxphones is still in-development. Follow these temporary instructions to try it:
Clone the repo, then copy the plugin file from `bd/builds` to your BD plugins folder. Then run `cargo run` from the `daemon` directory. Finally, enable the BD plugin.

### The below instructions do not work yet!
### Manual
Run:
```
./install.sh
```
This will install the daemon then copy the BetterDiscord plugin to your plugins folder.

### Updating
The client-side plugin updates through Discord. 

To update the daemon, run:
```
./updateDaemon.sh
```
