# Tuxphones

Discord screensharing audio for Linux.

Development is ongoing and therefore some features do not work fully or are broken. If you find a bug, open an issue!

## State of the Project
### Project Activity
We are very busy, so progress will be slow for the forseeable future.

### General Support
We are currently working on supporting PulseAudio and X11. Once these become stable, we will start working on PipeWire and Wayland.

### BetterDiscord Plugin
After the big Discord update in September 2022, the functionality of the plugin was completely broken. While we have tried to fix it as much as possible, it is still slightly inconsistent in reporting and integration so you may have to refresh Discord (Ctrl/Cmd+R) a few times to get it to load properly.

### Daemon
While most of the daemon works properly, we can only transmit video to Discord at the moment (through WebRTC). We are undergoing a transition from WebRTC to UDP transmission, so nothing transmits right now. To get a better look at the actual transmission code, look at our sister project [here](https://github.com/ImTheSquid/gst-discordsender).

### Contributions
We are open to contributions, however the project is still not stable so things will break. We plan to make an API for extending to PipeWire and Wayland in the future once we have a MVP.

## Installation
### Prerequisites
- BetterDiscord
- Rust
- Cargo
- Systemd
- PulseAudio Dev Libraries
- All GStreamer Dev Libraries

### Tuxphones is still in-development. Follow these temporary instructions to try it:
Clone the repo, then copy the plugin file from `bd/release` to your BD plugins folder. Then run `cargo run` from the `daemon` directory. Finally, enable the BD plugin.

### The below instructions do not work yet! The Crates package is very outdated and will not be updated until the project works.
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
