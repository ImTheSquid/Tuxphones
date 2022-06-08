import { unlinkSync, existsSync } from 'fs';
import { createServer, createConnection } from 'net';
import { join } from 'path';

const {Logger, Patcher, WebpackModules, DiscordModules, ContextMenu} = PluginApi;
const { Dispatcher } = DiscordModules;

const userMod = BdApi.findModuleByProps("getCurrentUser");

export default class extends BasePlugin {
    onStart() {
        // Make sure HOME is defined, Discord refuses to read files from XDG_RUNTIME_DIR
        if (!process.env.HOME) {
            BdApi.showToast('XDG_RUNTIME_DIR is not defined.', {type: 'error'});
            return;
        }

        this.sockPath = join(process.env.HOME, '.config', 'tuxphones.sock');
        this.serverSockPath = join(process.env.HOME, '.config', 'tuxphonesjs.sock');
        
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

        ContextMenu.getDiscordMenu('Confirm').then(m => {
            Patcher.after(m, 'default', (that, [arg], ret) => {
                Logger.log(that)
                Logger.log(arg)
                Logger.log(ret)
                if (!Array.isArray(ret.props.children)) return;
    
                if (arg.sound) {
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
                        res(vals.map(v => {
                            v.sound = v.id.startsWith('window') && e.apps.includes(v.id.split(':')[1]);
                            return v;
                        }));
                    }
    
                    Dispatcher.subscribe('TUX_APPS', f);
                    this.getInfo();
                }));
            });
        });

        // Patch stream start to interrupt if using sound
        Patcher.instead(Dispatcher, 'dirtyDispatch', (_, [arg], original) => {
            // Logger.log(arg)
            original(arg);
        });
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
                const {apps} = obj;
                break;
            case 'ConnectionId':
                const {id} = obj;
                break;
            default:
                Logger.err(`Received unknown command type: ${obj.type}`);
        }
    }

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

    getInfo() {
        // Test code
        Dispatcher.dirtyDispatch({
            type: 'TUX_APPS',
            apps: []
        })
        this.unixClient = createConnection(this.sockPath, () => {
            this.unixClient.write(JSON.stringify({
                type: 'GetInfo'
            }));
            this.unixClient.destroy();
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
    }
}