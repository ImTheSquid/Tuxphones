import { unlinkSync, existsSync } from 'fs';
import { createServer, createConnection } from 'net';
import { join } from 'path';

const {Logger, Patcher, WebpackModules, DiscordModules, ContextMenu} = PluginApi;
const { Dispatcher } = DiscordModules;
const React = BdApi.React;

const userMod = BdApi.findModuleByProps("getCurrentUser");
const Button = BdApi.findModuleByProps("BorderColors");
const colorStyles = BdApi.findModuleByProps("colorPrimary");

export default class extends BasePlugin {
    onStart() {
        // Make sure HOME is defined, Discord refuses to read files from XDG_RUNTIME_DIR
        if (!process.env.HOME) {
            BdApi.showToast('XDG_RUNTIME_DIR is not defined.', {type: 'error'});
            return;
        }

        this.sockPath = join(process.env.HOME, '.config', 'tuxphones.sock');
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

        // Hook WebSocket
        this.wsOnMessage = this.wsOnMessage.bind(this);
        this._onmessage = null;
        this._ws = null;

        Patcher.before(WebSocket.prototype, 'send', (that, [arg]) => {
            // Lock log to just video stream
            if (typeof(arg) !== 'string' || !that.url.includes('discord') || (this._ws && this._ws !== that)) return;

            const json = JSON.parse(arg);

            console.log('%cWS SEND FRAME ================================', 'color: green; font-size: large; margin-top: 20px;');

            // Check if stream has started, if so then hook onmessage
            if (json.op === 0 && json.d.streams.length > 0 && json.d.streams[0].type === 'screen' && json.d.user_id === userMod.getCurrentUser().id) {
                if (this._ws) {
                    this.resetVars();
                }
                this._ws = that;
                this._onmessage = that.onmessage;
                that.onmessage = this.wsOnMessage;
            } else if (json.op === 12 && json.d.video_ssrc !== 0 && json.d.rtx_ssrc !== 0) {
                console.log('%cRECEIVED SSRC INFORMATION', 'color: aqua; font-size: xx-large;');
                Logger.log('Video SSRC:');
                Logger.log(json.d.video_ssrc);
                Logger.log('RTX SSRC:');
                Logger.log(json.d.rtx_ssrc);

                this.ssrc = json.d.video_ssrc;
                const res = json.d.streams[0].max_resolution;
                this.resolution = {
                    width: res.width,
                    height: res.height,
                    is_fixed: res.type === 'fixed'
                };
            }

            Logger.log(json);
            console.log('%cWS END SEND FRAME ============================', 'color: green; font-size: large; margin-bottom: 20px;');
        });

        Patcher.before(WebSocket.prototype, 'close', (that, [arg]) => {
            Logger.log('CLOSE!');
            Logger.log(that);
            Logger.log(arg);
            if (this._ws === that) {
                console.log('%cSCREENSHARE CLOSED! Unlocking log...', 'color: red; font-size: x-large;');
                if (this._ws) {
                   this.resetVars();
                }
            }
        });

        ContextMenu.getDiscordMenu('GoLiveModal').then(m => {
            Patcher.after(m, 'default', (_, [arg], ret) => {
                //Logger.log(arg)
                Logger.log(ret)

                if (ret.props.children.props.children[2].props.children[1].props.activeSlide == 2 && ret.props.children.props.children[2].props.children[1].props.children[2].props.children.props.children.props.selectedSource?.sound) {
                    ret.props.children.props.children[2].props.children[2].props.children[0] = <div style={{'margin-right': '8px'}}>
                        {React.createElement(Button, {
                            onClick: () => {  },
                            size: Button.Sizes.SMALL
                        }, "Go Live with Sound")}
                    </div>
                }
                
                // ret.props.children.props.children[2].props.children[2].props.children[0] = <button style={{color: 'red'}}>GO LIVE</button>
                /*ret.props.children.props.children[2].props.children[2].props.children.splice(1, 0, <div style={{'margin-right': '8px'}}>
                    {React.createElement(Button, {
                        onClick: () => { this.clearAppCache() },
                        size: Button.Sizes.SMALL
                    }, "Refresh Tuxphones")}
                </div>);*/
            });
        });

        ContextMenu.getDiscordMenu('SourceSelect').then(m => {
            Patcher.after(m, 'default', (_, [arg], ret) => {
                //Logger.log(arg)
                //Logger.log(ret)
            });
        })

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

        this.getInfo([]);
    }

    resetVars() {
        this._ws.onmessage = this._onmessage;
        this._ws = null;
        this._onmessage = null;
    }

    wsOnMessage(m) {
        this._onmessage(m);

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
        }

        Logger.log(json);

        console.log('%cWS END RECV FRAME ============================', 'color: orange; font-size: large; margin-bottom: 20px;');
    }

    parseData(data) {
        let obj = JSON.parse(data);
        Logger.log(obj)
        switch (obj.type) {
            case 'ApplicationList':
                Dispatcher.dirtyDispatch({
                    type: 'TUX_APPS',
                    apps: obj.apps
                });
                break;
            case 'ConnectionId':
                const {id} = obj;
                break;
            default:
                Logger.err(`Received unknown command type: ${obj.type}`);
        }
    }

    /*clearAppCache() {
        this.unixClient = createConnection(this.sockPath, () => {
            this.unixClient.write(JSON.stringify({
                type: 'ClearAppCache'
            }));
            this.unixClient.destroy();
        });
    }*/

    startStream(ip, port, key, pid, resolution, ssrc) {
        this.unixClient = createConnection(this.sockPath, () => {
            this.unixClient.write(JSON.stringify({
                type: 'StartStream',
                ip: ip,
                port: port,
                key: key,
                pid: pid,
                resolution: resolution,
                ssrc: ssrc
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
            Dispatcher.dirtyDispatch({
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

        if (this._ws) {
            this.resetVars();
        }

        Patcher.unpatchAll();
    }
}