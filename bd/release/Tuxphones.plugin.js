/**
 * @name Tuxphones
 * @description Tuxphones
 * @version 0.1.0
 * @author ImTheSquid
 * @authorId 262055523896131584
 * @website https://github.com/ImTheSquid/Tuxphones
 * @source https://raw.githubusercontent.com/ImTheSquid/Tuxphones/main/plugin/Tuxphones.plugin.js
 */
/*@cc_on
@if (@_jscript)
    
    // Offer to self-install for clueless users that try to run this directly.
    var shell = WScript.CreateObject("WScript.Shell");
    var fs = new ActiveXObject("Scripting.FileSystemObject");
    var pathPlugins = shell.ExpandEnvironmentStrings("%APPDATA%\\BetterDiscord\\plugins");
    var pathSelf = WScript.ScriptFullName;
    // Put the user at ease by addressing them in the first person
    shell.Popup("It looks like you've mistakenly tried to run me directly. \n(Don't do that!)", 0, "I'm a plugin for BetterDiscord", 0x30);
    if (fs.GetParentFolderName(pathSelf) === fs.GetAbsolutePathName(pathPlugins)) {
        shell.Popup("I'm in the correct folder already.", 0, "I'm already installed", 0x40);
    } else if (!fs.FolderExists(pathPlugins)) {
        shell.Popup("I can't find the BetterDiscord plugins folder.\nAre you sure it's even installed?", 0, "Can't install myself", 0x10);
    } else if (shell.Popup("Should I copy myself to BetterDiscord's plugins folder for you?", 0, "Do you need some help?", 0x34) === 6) {
        fs.CopyFile(pathSelf, fs.BuildPath(pathPlugins, fs.GetFileName(pathSelf)), true);
        // Show the user where to put plugins in the future
        shell.Exec("explorer " + pathPlugins);
        shell.Popup("I'm installed!", 0, "Successfully installed", 0x40);
    }
    WScript.Quit();

@else@*/
const config = {
    info: {
        name: "Tuxphones",
        authors: [
            {
                name: "ImTheSquid",
                discord_id: "262055523896131584",
                github_username: "ImTheSquid",
                twitter_username: "ImTheSquid11"
            }
        ],
        version: "0.1.0",
        description: "Tuxphones",
        github: "https://github.com/ImTheSquid/Tuxphones",
        github_raw: "https://raw.githubusercontent.com/ImTheSquid/Tuxphones/main/plugin/Tuxphones.plugin.js"
    },
    main: "bundled.js"
};
class Dummy {
    constructor() {this._config = config;}
    start() {}
    stop() {}
}
 
if (!global.ZeresPluginLibrary) {
    BdApi.showConfirmationModal("Library Missing", `The library plugin needed for ${config.name ?? config.info.name} is missing. Please click Download Now to install it.`, {
        confirmText: "Download Now",
        cancelText: "Cancel",
        onConfirm: () => {
            require("request").get("https://betterdiscord.app/gh-redirect?id=9", async (err, resp, body) => {
                if (err) return require("electron").shell.openExternal("https://betterdiscord.app/Download?id=9");
                if (resp.statusCode === 302) {
                    require("request").get(resp.headers.location, async (error, response, content) => {
                        if (error) return require("electron").shell.openExternal("https://betterdiscord.app/Download?id=9");
                        await new Promise(r => require("fs").writeFile(require("path").join(BdApi.Plugins.folder, "0PluginLibrary.plugin.js"), content, r));
                    });
                }
                else {
                    await new Promise(r => require("fs").writeFile(require("path").join(BdApi.Plugins.folder, "0PluginLibrary.plugin.js"), body, r));
                }
            });
        }
    });
}
 
module.exports = !global.ZeresPluginLibrary ? Dummy : (([Plugin, Api]) => {
     const plugin = (Plugin, Library) => {
  const { Logger, Patcher, WebpackModules, DiscordModules, ContextMenu } = Library;
  const { Dispatcher, SelectedChannelStore, ButtonData, UserStore } = DiscordModules;
  const React = BdApi.React;
  const AuthenticationStore = Object.values(ZLibrary.WebpackModules.getAllModules()).find((m) => m.exports?.default?.getToken).exports.default;
  const RTCConnectionStore = BdApi.Webpack.getModule(BdApi.Webpack.Filters.byProps("getRTCConnectionId", "getWasEverRtcConnected"));
  const ChunkedRequests = BdApi.findModuleByProps("makeChunkedRequest");
  const WebSocketControl = BdApi.Webpack.getModule(BdApi.Webpack.Filters.byProps("lastTimeConnectedChanged")).getSocket();
  const GoLiveModal = BdApi.Webpack.getModule((m) => m.default?.toString().includes("GO_LIVE_MODAL"));
  const GetDesktopSources = BdApi.Webpack.getModule(BdApi.Webpack.Filters.byStrings("Can't get desktop sources outside of native app"), { defaultExport: false });
  function getFunctionNameFromString(obj, search) {
    for (const [k, v] of Object.entries(obj)) {
      if (search.every((str) => v?.toString().match(str))) {
        return k;
      }
    }
    return null;
  }
  return class extends Plugin {
    onStart() {
      this.webSocket = new WebSocket("ws://127.0.0.1:9000");
      this.webSocket.onmessage = this.parseData;
      this.webSocket.onerror = (_) => {
        BdApi.showConfirmationModal("Tuxphones Daemon Error", [
          "The Tuxphones daemon was not detected.\n",
          "If you don't know what this means or installed just the plugin and not the daemon, get help installing the daemon by going to the GitHub page:",
          /* @__PURE__ */ React.createElement("a", {
            href: "https://github.com/ImTheSquid/Tuxphones",
            target: "_blank"
          }, "Tuxphones Github"),
          " \n",
          `If you're sure you already installed the daemon, make sure it's running then click "Reload Discord".`
        ], {
          danger: true,
          confirmText: "Reload Discord",
          cancelText: "Stop Tuxphones",
          onConfirm: () => {
            location.reload();
          }
        });
      };
      this.webSocket.onopen = (_) => this.onOpen();
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
      Patcher.before(WebSocket.prototype, "send", (that, args) => {
        const arg = args[0];
        if (typeof arg !== "string" || !that.url.includes("discord") || this._ws && this._ws !== that)
          return;
        const json = JSON.parse(arg);
        console.log("%cWS SEND FRAME ================================", "color: green; font-size: large; margin-top: 20px;");
        if (json.op === 0 && json.d.streams.length > 0 && json.d.streams[0].type === "screen" && json.d.user_id === UserStore.getCurrentUser().id) {
          console.log("%cHOOKING SOCKET", "color: blue; font-size: xx-large;");
          if (this._ws) {
            this.resetVars();
          }
          this._ws = that;
          this._onmessage = that.onmessage;
          that.onmessage = this.wsOnMessage;
        } else if (json.op == 1 && this._ws === that) {
          json.d.data.mode = "xsalsa20_poly1305_lite";
          json.d.mode = "xsalsa20_poly1305_lite";
          args[0] = JSON.stringify(json);
        } else if (json.op == 5) {
          this.voice_ssrc = json.d.ssrc;
        }
        Logger.log(json);
        console.log("%cWS END SEND FRAME ============================", "color: green; font-size: large; margin-bottom: 20px;");
      });
      Patcher.before(WebSocket.prototype, "close", (that, [arg]) => {
        Logger.log("TUXPHONES CLOSE!");
        Logger.log(that);
        Logger.log(arg);
        if (this._ws === that) {
          console.log("%cSCREENSHARE CLOSED! Unlocking log...", "color: red; font-size: x-large;");
          if (this._ws) {
            this.resetVars();
          }
        }
      });
      Patcher.instead(Dispatcher, "dispatch", (_, [arg], original) => {
        if (this.interceptNextStreamServerUpdate && arg.type === "STREAM_SERVER_UPDATE") {
          Logger.log("STREAM SERVER UPDATE INTERCEPTED");
          Logger.log(arg);
          if (arg.streamKey) {
            this.streamKey = arg.streamKey;
          }
          WebSocketControl.streamSetPaused(this.streamKey, false);
          Logger.log(this.streamKey);
          this.startStream(this.currentSoundProfile.pid, this.currentSoundProfile.xid, this.selectedResolution, this.selectedFPS, this.ip, this.port, this.secret_key, this.voice_ssrc, this.base_ssrc);
        }
        return original(arg);
      });
      this.showTuxOk = false;
      if (GoLiveModal)
        this.patchGoLive(GoLiveModal);
      else {
        new Promise((resolve) => {
          const cancel = WebpackModules.addListener((module2) => {
            if (!module2.default?.toString().includes("GO_LIVE_MODAL"))
              return;
            resolve(module2);
            cancel();
          });
        }).then((m) => {
          this.patchGoLive(m);
        });
      }
      this.observer = new MutationObserver((mutations) => {
        if (mutations.filter((mut) => mut.addedNodes.length === 0 && mut.target.hasChildNodes()).length == 0)
          return;
        const res = mutations.flatMap((mut) => Array.from(mut.target.childNodes.values())).filter((node) => node.childNodes.length === 1).flatMap((node) => Array.from(node.childNodes.values())).filter((node) => node.nodeName === "DIV" && Array.from(node.childNodes.values()).some((node2) => node2.matches && node2.matches("[class*=flex]")))[0];
        if (res) {
          res.querySelector("[class*=flex]").innerText = this.showTuxOk ? "Tuxphones sound enabled!" : "Tuxphones not available.";
        }
      });
      this.observer.observe(document.querySelector("div > [class^=layerContainer]"), { childList: true, subtree: true });
      Patcher.after(GetDesktopSources, getFunctionNameFromString(GetDesktopSources, [/getDesktopCaptureSources/]), (_, __, ret) => {
        return ret.then((vals) => new Promise((res) => {
          const f = function dispatch(e) {
            Dispatcher.unsubscribe("TUX_APPS", dispatch);
            Logger.log("Found Sources:");
            Logger.log(vals);
            Logger.log("Found Sound Apps:");
            Logger.log(e.apps);
            res(vals.map((v) => {
              let found = e.apps.find((el) => el.xid === parseInt(v.id.split(":")[1]));
              if (v.id.startsWith("window") && found) {
                Logger.log(`Associating ${v.id} with sound profile for ${found.name}`);
                v.sound = found;
              } else {
                v.sound = null;
              }
              return v;
            }));
          };
          Dispatcher.subscribe("TUX_APPS", f);
          this.getInfo(vals.filter((v) => v.id.startsWith("window")).map((v) => parseInt(v.id.split(":")[1])));
        }));
      });
    }
    wsOnMessage(m) {
      const json = JSON.parse(m.data);
      console.log("%cWS RECV FRAME ================================", "color: orange; font-size: large; margin-top: 20px;");
      if (json.op === 4) {
        console.log("%cRECEIVED CODEC AND ENCRYPTION INFORMATION", "color: aqua; font-size: xx-large;");
        Logger.log("Audio Codec:");
        Logger.log(json.d.audio_codec);
        Logger.log("Encryption Mode:");
        Logger.log(json.d.mode);
        Logger.log("Secret key:");
        Logger.log(json.d.secret_key);
        this.secret_key = json.d.secret_key;
        return;
      } else if (json.op == 2) {
        this.base_ssrc = json.d.ssrc;
        this.ip = json.d.ip;
        this.port = json.d.port;
      }
      Logger.log(json);
      console.log("%cWS END RECV FRAME ============================", "color: orange; font-size: large; margin-bottom: 20px;");
      this._onmessage(m);
    }
    resetVars() {
      this._ws.onmessage = this._onmessage;
      this._ws = null;
      this._onmessage = null;
      this.currentSoundProfile = null;
      this.interceptNextStreamServerUpdate = false;
      this.base_ssrc = null;
      this.voice_ssrc = null;
    }
    patchGoLive(m) {
      Patcher.after(m, "default", (_, __, ret) => {
        Logger.log(ret);
        if (ret.props.children.props.children[2].props.children[1].props.activeSlide == 2) {
          if (ret.props.children.props.children[2].props.children[1].props.children[2].props.children.props.children.props.selectedSource.sound) {
            this.showTuxOk = true;
            ret.props.children.props.children[2].props.children[2].props.children[0] = /* @__PURE__ */ React.createElement("div", {
              style: { "margin-right": "8px" }
            }, React.createElement(ButtonData, {
              onClick: () => {
                const streamInfo = ret.props.children.props.children[2].props.children[1].props.children[2].props.children.props.children.props;
                this.currentSoundProfile = streamInfo.selectedSource.sound;
                this.selectedFPS = streamInfo.selectedFPS;
                this.selectedResolution = streamInfo.selectedResolution;
                Logger.log("Creating Sound Stream");
                this.createStream(streamInfo.guildId, SelectedChannelStore.getVoiceChannelId());
              },
              size: ButtonData.Sizes.SMALL
            }, "Go Live with Sound"));
          } else {
            this.showTuxOk = false;
          }
        }
      });
    }
    createStream(guild_id, channel_id) {
      this.interceptNextStreamServerUpdate = true;
      WebSocketControl.streamCreate(guild_id === null ? "call" : "guild", guild_id, channel_id, null);
    }
    parseData(msg) {
      let obj = JSON.parse(msg.data);
      Logger.log(obj);
      switch (obj.type) {
        case "ApplicationList":
          Dispatcher.dispatch({
            type: "TUX_APPS",
            apps: obj.apps
          });
          break;
        case "StreamPreview":
          Logger.log(this.streamKey);
          ChunkedRequests.makeChunkedRequest(`/streams/${this.streamKey}/preview`, {
            thumbnail: `data:image/jpeg;base64,${obj.jpg}`
          }, {
            method: "POST",
            token: AuthenticationStore.getToken()
          });
          break;
        default:
          Logger.err(`Received unknown command type: ${obj.type}`);
      }
    }
    startStream(pid, xid, selectedResolution, framerate, ip, port, secret_key, voice_ssrc, base_ssrc) {
      let resolution = null;
      switch (selectedResolution) {
        case 720:
          resolution = {
            width: 1280,
            height: 720,
            is_fixed: true
          };
          break;
        case 1080:
          resolution = {
            width: 1920,
            height: 1080,
            is_fixed: true
          };
          break;
        default:
          resolution = {
            width: 0,
            height: 0,
            is_fixed: false
          };
          break;
      }
      this.webSocket.send(JSON.stringify({
        type: "StartStream",
        pid,
        xid,
        resolution,
        framerate,
        rtc_connection_id: RTCConnectionStore.getRTCConnectionId(),
        secret_key,
        voice_ssrc,
        base_ssrc,
        ip,
        port
      }));
    }
    endStream() {
      this.webSocket.send(JSON.stringify({
        type: "StopStream"
      }));
    }
    getInfo(xids) {
      this.webSocket.send(JSON.stringify({
        type: "GetInfo",
        xids
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
  };
};
     return plugin(Plugin, Api);
})(global.ZeresPluginLibrary.buildPlugin(config));
/*@end@*/