import { createServer } from 'net';
import { createConnection } from 'net';
import { join } from 'path';

const {Logger} = PluginApi;

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
            Logger.log(sock);
        });

        this.endStream();
    }

    startStream(ip, port, key, pid, width, height, is_fixed, ssrc) {
        this.unixServer = createConnection(this.sockPath, () => {
            this.unixServer.write(JSON.stringify({
                type: 'StartStream',
                ip: ip,
                port: port,
                key: key,
                pid: pid,
                resolution: {
                    width: width,
                    height: height,
                    is_fixed: is_fixed
                },
                ssrc: ssrc
            }));
            this.unixServer.destroy();
        });
    }

    endStream() {
        this.unixServer = createConnection(this.sockPath, () => {
            this.unixServer.write(JSON.stringify({
                type: 'StopStream'
            }));
            this.unixServer.destroy();
        });
    }

    getInfo() {
        this.unixServer = createConnection(this.sockPath, () => {
            this.unixServer.write(JSON.stringify({
                type: 'GetInfo'
            }));
            this.unixServer.destroy();
        });
    }

    onStop() {
        if (this.unixServer && this.unixServer.listening) {
            this.unixServer.close();
        }
    }
}