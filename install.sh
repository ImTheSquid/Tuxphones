#!/bin/sh

cargo install tuxphones

BASEDIR=$(dirname "$0")
mkdir -p "$HOME"/.local/share/systemd/user/
cp "$BASEDIR"/Tuxphones.service "$HOME"/.local/share/systemd/user/
systemctl --user enable Tuxphones.service
systemctl --user start Tuxphones.service

cp "$BASEDIR"/bd/builds/Tuxphones.plugin.js "$HOME"/.config/BetterDiscord/plugins/