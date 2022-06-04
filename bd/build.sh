#!/bin/sh

./node_modules/.bin/bdbuilder --plugin="./Tuxphones" --production
cp builds/Tuxphones.plugin.js ~/.config/BetterDiscord/plugins