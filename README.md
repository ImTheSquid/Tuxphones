# Tuxphones

Discord screensharing audio for Linux.

## Installation
### Prerequisites
- BetterDiscord
- Rust
- Cargo
- Systemd
- PulseAudio Dev Libraries
- All GStreamer Dev Libraries

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