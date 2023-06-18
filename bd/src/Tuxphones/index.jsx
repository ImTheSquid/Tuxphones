module.exports = (Plugin, Library) => {
const {Logger, Patcher, WebpackModules, DiscordModules, ContextMenu} = Library;
const { Dispatcher, SelectedChannelStore, ButtonData, UserStore } = DiscordModules;
const React = BdApi.React;

// Useful modules maybe: ApplicationStreamingSettingsStore, ApplicationStreamingStore
const AuthenticationStore = Object.values(ZLibrary.WebpackModules.getAllModules()).find(m => m.exports?.default?.getToken).exports.default; // Works (should be replaced with custom solution eventually)
const RTCConnectionStore = BdApi.Webpack.getModule(BdApi.Webpack.Filters.byProps("getRTCConnectionId", "getWasEverRtcConnected"));
//const WebRequests = BdApi.Webpack.getModule(BdApi.Webpack.Filters.byProps("getXHR"));
const ChunkedRequests = BdApi.findModuleByProps("makeChunkedRequest");
//const RTCControlSocket = BdApi.Webpack.getModule(m => m.Z?.prototype?.connect);
const WebSocketControl = BdApi.Webpack.getModule(BdApi.Webpack.Filters.byProps("lastTimeConnectedChanged")).getSocket();
const GoLiveModal = BdApi.Webpack.getModule(m => m.default?.toString().includes("GO_LIVE_MODAL"));
// const DesktopSourcesChecker = BdApi.Webpack.getModule(BdApi.Webpack.Filters.byProps("installedLogHooks")).prototype;
const GetDesktopSources = BdApi.Webpack.getModule(BdApi.Webpack.Filters.byStrings("Can't get desktop sources outside of native app"), {defaultExport: false});

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

        this.wsOnMessage = this.wsOnMessage.bind(this);
        this._onmessage = null;
        this._ws = null;
    }

    onOpen() {
        Patcher.before(WebSocket.prototype, 'send', (that, args) => {
            const arg = args[0];
            if (typeof(arg) !== 'string' || !that.url.includes('discord') || (this._ws && this._ws !== that)) return;

            const json = JSON.parse(arg);

            console.log('%cWS SEND FRAME ================================', 'color: green; font-size: large; margin-top: 20px;');

            // Condition for voice stream: json.op === 0 && json.d.streams.length > 0 && json.d.streams[0].type === 'video' && json.d.user_id === UserStore.getCurrentUser().id

            // Check if stream has started, if so then hook onmessage
            if (json.op === 0 && json.d.streams.length > 0 && json.d.streams[0].type === 'screen' && json.d.user_id === UserStore.getCurrentUser().id) {
                console.log('%cHOOKING SOCKET', 'color: blue; font-size: xx-large;');
                if (this._ws) {
                    this.resetVars();
                }
                this._ws = that;
                this._onmessage = that.onmessage;
                that.onmessage = this.wsOnMessage;
                // this.token = json.d.token;
            } else if (json.op == 1 && this._ws === that) {
                json.d.data.mode = 'xsalsa20_poly1305_lite';
                json.d.mode = 'xsalsa20_poly1305_lite';
                args[0] = JSON.stringify(json);
            } else if (json.op == 5) {
                // WARNING WARNING WARNING ==========================================================================
                // This is a hack, it may not always work!
                // Still need to test this in multi-person VC
                this.voice_ssrc = json.d.ssrc;
            }
            // else if (json.op === 12 && json.d.video_ssrc !== 0 && json.d.rtx_ssrc !== 0) {
            //     console.log('%cRECEIVED SSRC INFORMATION', 'color: aqua; font-size: xx-large;');
            //     Logger.log('Video SSRC:');
            //     Logger.log(json.d.video_ssrc);
            //     Logger.log('RTX SSRC:');
            //     Logger.log(json.d.rtx_ssrc);

            //     this.ssrc = json.d.video_ssrc;
            //     const res = json.d.streams[0].max_resolution;
            //     this.resolution = {
            //         width: res.width,
            //         height: res.height,
            //         is_fixed: res.type === 'fixed'
            //     };
            // }

            Logger.log(json);
            console.log('%cWS END SEND FRAME ============================', 'color: green; font-size: large; margin-bottom: 20px;');
        });

        Patcher.before(WebSocket.prototype, 'close', (that, [arg]) => {
            Logger.log('TUXPHONES CLOSE!');
            Logger.log(that);
            Logger.log(arg);
            if (this._ws === that) {
                console.log('%cSCREENSHARE CLOSED! Unlocking log...', 'color: red; font-size: x-large;');
                if (this._ws) {
                   this.resetVars();
                }
            }
        });

        Patcher.instead(Dispatcher, 'dispatch', (_, [arg], original) => {
            if (this.interceptNextStreamServerUpdate && arg.type === 'STREAM_SERVER_UPDATE') {
                Logger.log("STREAM SERVER UPDATE INTERCEPTED");
                Logger.log(arg)
                // let res = null;
                // switch (this.selectedResolution) {
                //     case 720: res = {
                //         width: 1280,
                //         height: 720,
                //         is_fixed: true
                //     };
                //         break;
                //     case 1080: res = {
                //         width: 1920,
                //         height: 1080,
                //         is_fixed: true
                //     };
                //         break;
                //     default: res = {
                //         width: 0,
                //         height: 0,
                //         is_fixed: false
                //     };
                //         break;
                // }

                if (arg.streamKey) {
                    this.streamKey = arg.streamKey;
                }
                WebSocketControl.streamSetPaused(this.streamKey, false);
                Logger.log(this.streamKey)
                // this.startStream(this.currentSoundProfile.pid, this.currentSoundProfile.xid, this.selectedResolution, this.selectedFPS, this.ip, this.port, this.secret_key, this.voice_ssrc, this.base_ssrc);

                // this.startStream(this.currentSoundProfile.pid, this.currentSoundProfile.xid, res, this.selectedFPS, this.serverId, arg.token, arg.endpoint);
                // return new Promise(res => res());
            }
            // } else if (this.currentSoundProfile) {
            //     // Hide the stream's existence from Discord until ready to test Tuxphones/Discord interaction
            //     switch (arg.type) {
            //         case 'STREAM_CREATE':
            //             Logger.log("SOUND SC PROFILE");
            //             Logger.log(arg);
            //             this.serverId = arg.rtcServerId;
            //             break;
            //             // return new Promise(res => res());
            //         case 'STREAM_UPDATE':
            //             Logger.log("SOUND SU PROFILE");
            //             Logger.log(arg);
            //             // this.streamKey = arg.streamKey;
            //             break;
            //             // return new Promise(res => res());
            //         case 'VOICE_STATE_UPDATES':
            //             Logger.log("SOUND VSU PROFILE");
            //             Logger.log(arg);
            //             arg.voiceStates[0].selfStream = false;
            //             break;
            //     }
            // } else if (arg.type.match(/(STREAM.*_UPDATE|STREAM_CREATE)/)) {
            //     Logger.log("STREAM CREATE OR UPDATE");
            //     Logger.log(arg);
            // }else {
            //     // Logger.log(arg)
            // }
            return original(arg);
        });

        this.showTuxOk = false;

        if (GoLiveModal) this.patchGoLive(GoLiveModal)
        else {
            new Promise(resolve => {
                const cancel = WebpackModules.addListener(module => {
                    if (!module.default?.toString().includes("GO_LIVE_MODAL")) return;
                    resolve(module);
                    cancel();
                });
            }).then(m => {
                this.patchGoLive(m);
            });
        }

        this.observer = new MutationObserver(mutations => {
            if (mutations.filter(mut => mut.addedNodes.length === 0 && mut.target.hasChildNodes()).length == 0) return;

            const res = mutations
                .flatMap(mut => Array.from(mut.target.childNodes.values()))
                .filter(node => node.childNodes.length === 1)
                .flatMap(node => Array.from(node.childNodes.values()))
                .filter(node => node.nodeName === "DIV" && Array.from(node.childNodes.values())
                .some(node => node.matches && node.matches("[class*=flex]")))[0];

            if (res) {
                res.querySelector("[class*=flex]").innerText = this.showTuxOk ? "Tuxphones sound enabled!" : "Tuxphones not available.";
            }
        });

        this.observer.observe(document.querySelector("div > [class^=layerContainer]"), {childList: true, subtree: true});

        // Add extra info to desktop sources list
        Patcher.after(GetDesktopSources, getFunctionNameFromString(GetDesktopSources, [/getDesktopCaptureSources/]), (_, __, ret) => {
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
        // Patcher.after(RTCControlSocket.Z.prototype, '_handleReady', (that, _, __) => {
        //     Logger.log("handling ready")
        //     that._connection.on("connected", (___, info) => {
        //         Logger.log(info)
        //         this.ip = info.address;
        //     });
        // });
    }

    wsOnMessage(m) {
        const json = JSON.parse(m.data);

        console.log('%cWS RECV FRAME ================================', 'color: orange; font-size: large; margin-top: 20px;');

        if (json.op === 4) {
            console.log('%cRECEIVED CODEC AND ENCRYPTION INFORMATION', 'color: aqua; font-size: xx-large;');
            Logger.log('Audio Codec:');
            Logger.log(json.d.audio_codec);
            Logger.log('Encryption Mode:');
            Logger.log(json.d.mode);
            Logger.log('Secret key:');
            Logger.log(json.d.secret_key);
            this.secret_key = json.d.secret_key;
            this.startStream(this.currentSoundProfile.pid, this.currentSoundProfile.xid, this.selectedResolution, this.selectedFPS, this.ip, this.port, this.secret_key, this.voice_ssrc, this.base_ssrc);
            return; // Disallow encryption information, stopping the stream from being created
        } else if (json.op == 2) {
            this.base_ssrc = json.d.ssrc;
            this.ip = json.d.ip;
            this.port = json.d.port;
        }

        Logger.log(json);

        console.log('%cWS END RECV FRAME ============================', 'color: orange; font-size: large; margin-bottom: 20px;');

        this._onmessage(m);
    }

    resetVars() {
        this.endStream();
        this._ws.onmessage = this._onmessage;
        this._ws = null;
        this._onmessage = null;
        this.currentSoundProfile = null;
        this.interceptNextStreamServerUpdate = false;
        this.base_ssrc = null;
        this.voice_ssrc = null;
    }

    patchGoLive(m) {
        Patcher.after(m, 'default', (_, __, ret) => {
            Logger.log(ret)

            if (ret.props.children.props.children[2].props.children[1].props.activeSlide == 2) {
                if (ret.props.children.props.children[2].props.children[1].props.children[2].props.children.props.children.props.selectedSource.sound) {
                    this.showTuxOk = true;
                    ret.props.children.props.children[2].props.children[2].props.children[0] = <div style={{'margin-right': '8px'}}>
                        {React.createElement(ButtonData, {
                            onClick: () => {
                                const streamInfo = ret.props.children.props.children[2].props.children[1].props.children[2].props.children.props.children.props;
                                this.currentSoundProfile = streamInfo.selectedSource.sound;
                                this.selectedFPS = streamInfo.selectedFPS;
                                this.selectedResolution = streamInfo.selectedResolution;
                                Logger.log("Creating Sound Stream");
                                this.createStream(streamInfo.guildId, SelectedChannelStore.getVoiceChannelId());
                            },
                            size: ButtonData.Sizes.SMALL
                        }, "Go Live with Sound")}
                    </div>
                } else {
                    this.showTuxOk = false;
                }
            }
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
                Logger.log(this.streamKey)
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
    startStream(pid, xid, selectedResolution, framerate, ip, port, secret_key, voice_ssrc, base_ssrc) {
        let resolution = null;
        switch (selectedResolution) {
            case 720: resolution = {
                width: 1280,
                height: 720,
                is_fixed: true
            };
                break;
            case 1080: resolution = {
                width: 1920,
                height: 1080,
                is_fixed: true
            };
                break;
            default: resolution = {
                width: 0,
                height: 0,
                is_fixed: false
            };
                break;
        }

        this.webSocket.send(JSON.stringify({
            type: 'StartStream',
            pid: pid,
            xid: xid,
            resolution: resolution,
            framerate: framerate,
            // server_id: server_id,
            // user_id: AuthenticationStore.getId(),
            // token: token,
            // session_id: AuthenticationStore.getSessionId(), // getSessionId [no], getMediaSessionId [no], getRemoteSessionId [no], getActiveMediaSessionId [no]
            rtc_connection_id: RTCConnectionStore.getRTCConnectionId(),
            secret_key: secret_key,
            voice_ssrc: voice_ssrc,
            base_ssrc: base_ssrc,
            ip: ip,
            port: port,
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
        if (this._ws) {
            this.resetVars();
        }
        Patcher.unpatchAll();
        if (this.observer)
            this.observer.disconnect();
    }
}
}