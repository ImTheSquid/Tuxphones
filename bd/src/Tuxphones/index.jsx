module.exports = (Plugin, Library) => {
const {Logger, Patcher, WebpackModules, DiscordModules, ContextMenu} = Library;
const { Dispatcher, SelectedChannelStore, ButtonData } = DiscordModules;
const React = BdApi.React;

// Useful modules maybe: ApplicationStreamingSettingsStore, ApplicationStreamingStore
const AuthenticationStore = Object.values(ZLibrary.WebpackModules.getAllModules()).find(m => m.exports?.default?.getToken).exports.default; // Works (should be replaced with custom solution eventually)
const RTCConnectionStore = BdApi.Webpack.getModule(BdApi.Webpack.Filters.byProps("getRTCConnectionId", "getWasEverRtcConnected"));
const WebRequests = BdApi.Webpack.getModule(BdApi.Webpack.Filters.byProps("getXHR"));
const ChunkedRequests = BdApi.findModuleByProps("makeChunkedRequest");
const RTCControlSocket = BdApi.Webpack.getModule(m => m.Z?.prototype?.connect);
const WebSocketControl = BdApi.Webpack.getModule(BdApi.Webpack.Filters.byProps("lastTimeConnectedChanged")).getSocket();
const GoLiveModal = BdApi.Webpack.getModule(m => m.default?.toString().includes("GO_LIVE_MODAL"));
const GetDesktopSourcesMod = BdApi.Webpack.getModule(m => Object.values(m).filter(v => v).some(v => v.SCREEN && v.WINDOW));

function getFunctionNameFromString(obj, search) {
    for (const [k, v] of Object.entries(obj)) {
        if (search.every(str => v?.toString().match(str))) {
            return k;
        }
    }
    return null;
}

return class extends Plugin {
    onStart() {
        this.webSocket = new WebSocket("ws://127.0.0.1:9000");
        this.webSocket.onmessage = this.parseData;
        this.webSocket.onerror = _ => {
            BdApi.showConfirmationModal('Tuxphones Daemon Error', [
                'The Tuxphones daemon was not detected.\n',
                'If you don\'t know what this means or installed just the plugin and not the daemon, get help installing the daemon by going to the GitHub page:',
                <a href='https://github.com/ImTheSquid/Tuxphones' target='_blank'>Tuxphones Github</a>,
                ' \n',
                'If you\'re sure you already installed the daemon, make sure it\'s running then click "Reload Discord".'
            ], {
                danger: true,
                confirmText: 'Reload Discord',
                cancelText: 'Stop Tuxphones',
                onConfirm: () => {
                    location.reload();
                }
            })
        }

        this.webSocket.onopen = _ => this.onOpen();

        // Hook Dispatcher for when to intercept
        this.interceptNextStreamServerUpdate = false;
        this.currentSoundProfile = null;
        this.selectedFPS = null;
        this.selectedResolution = null;
        this.serverId = null;
        this.ip = null;
    }

    onOpen() {
        Patcher.instead(Dispatcher, 'dispatch', (_, [arg], original) => {
            if (this.interceptNextStreamServerUpdate && arg.type === 'STREAM_SERVER_UPDATE') {
                let res = null;
                switch (this.selectedResolution) {
                    case 720: res = {
                        width: 1280,
                        height: 720,
                        is_fixed: true
                    };
                        break;
                    case 1080: res = {
                        width: 1920,
                        height: 1080,
                        is_fixed: true
                    };
                        break;
                    default: res = {
                        width: 0,
                        height: 0,
                        is_fixed: false
                    };
                        break;
                }

                this.streamKey = arg.streamKey;
                WebSocketControl.streamSetPaused(this.streamKey, false);

                this.startStream(this.currentSoundProfile.pid, this.currentSoundProfile.xid, res, this.selectedFPS, this.serverId, arg.token, arg.endpoint, this.ip);
                return new Promise(res => res());
            } else if (this.currentSoundProfile) {
                // Hide the stream's existence from Discord until ready to test Tuxphones/Discord interaction
                switch (arg.type) {
                    case 'STREAM_CREATE':
                        this.serverId = arg.rtcServerId;
                        return new Promise(res => res());
                    case 'STREAM_UPDATE':
                        // this.streamKey = arg.streamKey;
                        return new Promise(res => res());
                    case 'VOICE_STATE_UPDATES':
                        arg.voiceStates[0].selfStream = false;
                        break;
                }
            } else if (arg.type.match(/(STREAM.*_UPDATE|STREAM_CREATE)/)) {
                Logger.log(arg)
            }else {
                // Logger.log(arg)
            }
            return original(arg);
        });

        let showTuxOk = false;

        Patcher.after(GoLiveModal, 'default', (_, __, ret) => {
            Logger.log(ret)

            if (ret.props.children.props.children[2].props.children[1].props.activeSlide == 2) {
                if (ret.props.children.props.children[2].props.children[1].props.children[2].props.children.props.children.props.selectedSource.sound) {
                    showTuxOk = true;
                    ret.props.children.props.children[2].props.children[2].props.children[0] = <div style={{'margin-right': '8px'}}>
                        {React.createElement(ButtonData, {
                            onClick: () => {
                                const streamInfo = ret.props.children.props.children[2].props.children[1].props.children[2].props.children.props.children.props;
                                this.currentSoundProfile = streamInfo.selectedSource.sound;
                                this.selectedFPS = streamInfo.selectedFPS;
                                this.selectedResolution = streamInfo.selectedResolution;
                                this.createStream(streamInfo.guildId, SelectedChannelStore.getVoiceChannelId());
                            },
                            size: ButtonData.Sizes.SMALL
                        }, "Go Live with Sound")}
                    </div>
                } else {
                    showTuxOk = false;
                }
            }
        });

        // Add extra info to desktop sources list
        Patcher.after(GetDesktopSourcesMod, getFunctionNameFromString(GetDesktopSourcesMod, [/getDesktopCaptureSources/]), (_, __, ret) => {
            return ret.then(vals => new Promise(res => {
                const f = function dispatch(e) {
                    Dispatcher.unsubscribe('TUX_APPS', dispatch);

                    // Check against window IDs to see if comaptible with sound
                    Logger.log("Found Sources:")
                    Logger.log(vals);
                    Logger.log("Found Sound Apps:")
                    Logger.log(e.apps);
                    res(vals.map(v => {
                        let found = e.apps.find(el => el.xid === parseInt(v.id.split(':')[1]));
                        if (v.id.startsWith('window') && found) {
                            Logger.log(`Associating ${v.id} with sound profile for ${found.name}`)
                            v.sound = found;
                        } else {
                            v.sound = null;
                        }
                        return v;
                    }));
                }

                Dispatcher.subscribe('TUX_APPS', f);
                this.getInfo(vals.filter(v => v.id.startsWith('window')).map(v => parseInt(v.id.split(':')[1])));
            }));
        });

        // Patch stream to get IP address
        Patcher.after(RTCControlSocket.Z.prototype, '_handleReady', (that, _, __) => {
            that._connection.on("connected", (___, info) => {
                Logger.log(info)
                this.ip = info.address;
            });
        });
    }

    createStream(guild_id, channel_id) {
        this.interceptNextStreamServerUpdate = true;
        WebSocketControl.streamCreate(
            guild_id === null ? 'call' : 'guild', // type
            guild_id, // guild_id
            channel_id, // channel or DM id
            null, // preferred_region
        );
    }

    parseData(msg) {
        let obj = JSON.parse(msg.data);
        Logger.log(obj)
        switch (obj.type) {
            case 'ApplicationList':
                Dispatcher.dispatch({
                    type: 'TUX_APPS',
                    apps: obj.apps
                });
                break;
            case 'StreamPreview':
                // Alternatively, DiscordNative.http.makeChunkedRequest
                ChunkedRequests.makeChunkedRequest(`/streams/${this.streamKey}/preview`, {
                    thumbnail: `data:image/jpeg;base64,${obj.jpg}` // May have to include charset?
                }, {
                    method: 'POST',
                    token: AuthenticationStore.getToken()
                });
                break;
            default:
                Logger.err(`Received unknown command type: ${obj.type}`);
        }
    }

    // server_id PRIORITY: RTC Server ID -> Guild ID -> Channel ID
    // Guild ID will always exist, so get RTC Server ID
    startStream(pid, xid, resolution, framerate, server_id, token, endpoint, ip) {
        this.webSocket.send(JSON.stringify({
            type: 'StartStream',
            pid: pid,
            xid: xid,
            resolution: resolution,
            framerate: framerate,
            server_id: server_id,
            user_id: AuthenticationStore.getId(),
            token: token,
            session_id: AuthenticationStore.getSessionId(), // getSessionId [no], getMediaSessionId [no], getRemoteSessionId [no], getActiveMediaSessionId [no]
            rtc_connection_id: RTCConnectionStore.getRTCConnectionId(),
            endpoint: endpoint,
            ip: ip,
            ice: {
                type: "IceData",
                urls: ['stun.l.google.com:19302'],
                username: '',
                credential: '',
                ttl: '',
            }
        }));
    }

    endStream() {
        this.webSocket.send(JSON.stringify({
            type: 'StopStream'
        }));
    }

    getInfo(xids) {
        this.webSocket.send(JSON.stringify({
            type: 'GetInfo',
            xids: xids
        }));
    }

    onStop() {
        this.webSocket.close();
        Patcher.unpatchAll();
    }
}
}