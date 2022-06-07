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
            let data = [];
            sock.on('data', d => data += d);
            sock.on('end', () =>{
                this.parseData(data);
                data = [];
            })
        });

        this.unixServer.listen(this.serverSockPath, () => Logger.log('Server bound'));

        this.endStream();
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

    startStream(ip, port, key, pid, width, height, is_fixed, ssrc) {
        this.unixClient = createConnection(this.sockPath, () => {
            this.unixClient.write(JSON.stringify({
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
    }
}