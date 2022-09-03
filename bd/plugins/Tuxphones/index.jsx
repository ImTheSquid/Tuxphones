import { unlinkSync, existsSync } from 'fs';
import { createServer, createConnection } from 'net';
import { join } from 'path';

const {Logger, Patcher, WebpackModules, DiscordModules, ContextMenu} = PluginApi;
const { Dispatcher } = DiscordModules;
const React = BdApi.React;

// Useful modules maybe: ApplicationStreamingSettingsStore, ApplicationStreamingStore
const AuthenticationStore = BdApi.findModule((m => m.default?.getToken)).default;
const RTCConnectionStore = BdApi.Webpack.getModule(BdApi.Webpack.Filters.byProps("getRTCConnectionId", "getWasEverRtcConnected"));
const UserStatusStore = BdApi.findModule((m => m.default?.getVoiceChannelId)).default;
const WebRequests = BdApi.findModule(m => m.default.get && m.default.post && m.default.put && m.default.patch && m.default.delete).default;
const ChunkedRequests = BdApi.findModuleByProps("makeChunkedRequest");
const RTCControlSocket = BdApi.findModuleByPrototypes("handleHello");
const WebSocketControl = BdApi.findModuleByPrototypes("streamCreate");
const Button = BdApi.findModuleByProps("BorderColors");

export default class extends BasePlugin {
    onStart() {
        // Make sure HOME is defined, Discord refuses to read files from XDG_RUNTIME_DIR
        if (!process.env.HOME) {
            BdApi.showToast('$HOME is not defined. Reload Discord after defining.', {type: 'error'});
            throw '$HOME is not defined.';
        }

        this.sockPath = join(process.env.HOME, '.config', 'tuxphones.sock');
        if (!existsSync(this.sockPath)) {
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
            throw 'Daemon not running!';
        }

        this.serverSockPath = join(process.env.HOME, '.config', 'tuxphonesjs.sock');

        if (existsSync(this.serverSockPath)) {
            unlinkSync(this.serverSockPath);
        }

        this.unixServer = createServer(sock => {
            let data = [];
            sock.on('data', d => data += d);
            sock.on('end', () =>{
                this.parseData(data);
                data = [];
            })
        });

        this.unixServer.listen(this.serverSockPath, () => Logger.log('Server bound'));

        // Hook Dispatcher for when to intercept
        this.interceptNextStreamServerUpdate = false;
        this.currentSoundProfile = null;
        this.selectedFPS = null;
        this.selectedResolution = null;
        this.serverId = null;
        this.webSocketControlObj = null;
        this.ip = null;

        Patcher.before(WebSocketControl.prototype, "_handleDispatch", (that, _, __) => {
            this.webSocketControlObj = that;
        })

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
                this.webSocketControlObj.streamSetPaused(this.streamKey, false);

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

        ContextMenu.getDiscordMenu('GoLiveModal').then(m => {
            Patcher.after(m, 'default', (_, __, ret) => {
                //Logger.log(arg)
                Logger.log(ret)

                if (ret.props.children.props.children[2].props.children[1].props.activeSlide == 2 && ret.props.children.props.children[2].props.children[1].props.children[2].props.children.props.children.props.selectedSource?.sound) {
                    ret.props.children.props.children[2].props.children[2].props.children[0] = <div style={{'margin-right': '8px'}}>
                        {React.createElement(Button, {
                            onClick: () => {
                                const streamInfo = ret.props.children.props.children[2].props.children[1].props.children[2].props.children.props.children.props;
                                this.currentSoundProfile = streamInfo.selectedSource.sound;
                                this.selectedFPS = streamInfo.selectedFPS;
                                this.selectedResolution = streamInfo.selectedResolution;
                                // this.serverId = streamInfo.guildId;
                                this.createStream(streamInfo.guildId, UserStatusStore.getVoiceChannelId());
                            },
                            size: Button.Sizes.SMALL
                        }, "Go Live with Sound")}
                    </div>
                }
            });
        });

        ContextMenu.getDiscordMenu('Confirm').then(m => {
            Patcher.after(m, 'default', (_, [arg], ret) => {
                if (!Array.isArray(ret.props.children)) return;
                Logger.log(arg)
                //Logger.log(ret)

                if (arg.selectedSource.sound) {
                    ret.props.children[1] = <p style={{color: 'green', padding: '0px 16px'}}>Tuxphones sound enabled!</p>
                } else {
                    ret.props.children[1] = <p style={{color: 'red', padding: '0px 16px'}}>Tuxphones not available.</p>
                }
            });
        });

        // Add extra info to desktop sources list
        // Stolen from https://rauenzi.github.io/BDPluginLibrary/docs/ui_discordcontextmenu.js.html#line-269, removed code that limited to default
        new Promise(resolve => {
            const cancel = WebpackModules.addListener(module => {
                if (!module.default || !module.DesktopSources) return;
                resolve(module);
                cancel();
            })}).then(m => {
            Patcher.after(m, 'default', (_, __, ret) => {
                return ret.then(vals => new Promise(res => {
                    const f = function dispatch(e) {
                        Dispatcher.unsubscribe('TUX_APPS', dispatch);

                        // Check against window IDs to see if comaptible with sound
                        Logger.log(vals);
                        Logger.log(e.apps);
                        res(vals.map(v => {
                            let found = e.apps.find(el => el.xid == v.id.split(':')[1]);
                            if (v.id.startsWith('window') && found) {
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
        });

        // Patch stream to get IP address
        Patcher.before(RTCControlSocket.prototype, 'send', (_, [op, d]) => {
            if (op === 1) {
                this.ip = d.address;
            }
        })
    }

    createStream(guild_id, channel_id) {
        this.interceptNextStreamServerUpdate = true;
        this.webSocketControlObj.streamCreate(
            guild_id === null ? 'call' : 'guild', // type
            guild_id, // guild_id
            channel_id, // channel or DM id
            null, // preferred_region
        );
    }

    parseData(data) {
        let obj = JSON.parse(data);
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
        this.unixClient = createConnection(this.sockPath, async () => {
            const {servers, ttl} = (await WebRequests.get({url: '/voice/ice'})).body;
            const authData = servers.find(server => server.credential);

            this.unixClient.write(JSON.stringify({
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
                    urls: servers.map(server => server.url),
                    username: authData.username,
                    credential: authData.credential,
                    ttl,
                }
            }));
            this.unixClient.destroy();
        });
    }

    endStream() {
        this.unixClient = createConnection(this.sockPath, () => {
            this.unixClient.write(JSON.stringify({
                type: 'StopStream'
            }));
            this.unixClient.destroy();
        });
    }

    getInfo(xids) {
        this.unixClient = createConnection(this.sockPath, () => {
            this.unixClient.write(JSON.stringify({
                type: 'GetInfo',
                xids: xids
            }));
            this.unixClient.destroy();
        });
        this.unixClient.on('error', e => {
            Logger.err(`[GetInfo] Socket client error: ${e}`);
            Dispatcher.dispatch({
                type: 'TUX_APPS',
                apps: []
            });
        });
    }

    onStop() {
        if (this.unixServer && this.unixServer.listening) {
            this.unixServer.close();
        }

        if (existsSync(this.serverSockPath)) {
            unlinkSync(this.serverSockPath);
        }

        Patcher.unpatchAll();
    }
}