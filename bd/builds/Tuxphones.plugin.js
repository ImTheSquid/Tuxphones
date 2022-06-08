/**
 * @name Tuxphones
 * @author ImTheSquid
 * @version 0.1.0
 * @description Tuxphones
 * @source https://github.com/ImTheSquid/Tuxphones
 * @updateUrl https://raw.githubusercontent.com/ImTheSquid/Tuxphones/main/plugin/Tuxphones.plugin.js
 */
/*@cc_on
@if (@_jscript)
    
    // Offer to self-install for clueless users that try to run this directly.
    var shell = WScript.CreateObject("WScript.Shell");
    var fs = new ActiveXObject("Scripting.FileSystemObject");
    var pathPlugins = shell.ExpandEnvironmentStrings("%APPDATA%\BetterDiscord\plugins");
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
/* Generated Code */
const config = {
	"info": {
		"name": "Tuxphones",
		"authors": [{
			"name": "ImTheSquid",
			"discord_id": "262055523896131584",
			"github_username": "ImTheSquid",
			"twitter_username": "ImTheSquid11"
		}],
		"version": "0.1.0",
		"description": "Tuxphones",
		"github": "https://github.com/ImTheSquid/Tuxphones",
		"github_raw": "https://raw.githubusercontent.com/ImTheSquid/Tuxphones/main/plugin/Tuxphones.plugin.js"
	},
	"build": {
		"copy": true,
		"zlibrary": true,
		"production": false,
		"alias": {}
	}
};
function buildPlugin([BasePlugin, PluginApi]) {
	const module = {
		exports: {}
	};
	(() => {
		"use strict";
		class StyleLoader {
			static styles = "";
			static element = null;
			static append(module, css) {
				this.styles += `/* ${module} */\n${css}`;
			}
			static inject(name = config.info.name) {
				if (this.element) this.element.remove();
				this.element = document.head.appendChild(Object.assign(document.createElement("style"), {
					id: name,
					textContent: this.styles
				}));
			}
			static remove() {
				if (this.element) {
					this.element.remove();
					this.element = null;
				}
			}
		}
		function ___createMemoize___(instance, name, value) {
			value = value();
			Object.defineProperty(instance, name, {
				value,
				configurable: true
			});
			return value;
		};
		const Modules = {
			get 'react-spring'() {
				return ___createMemoize___(this, 'react-spring', () => BdApi.findModuleByProps('useSpring'))
			},
			'@discord/utils': {
				get 'joinClassNames'() {
					return ___createMemoize___(this, 'joinClassNames', () => BdApi.findModule(m => typeof m?.default?.default === 'function')?.default)
				},
				get 'useForceUpdate'() {
					return ___createMemoize___(this, 'useForceUpdate', () => BdApi.findModuleByProps('useForceUpdate')?.useForceUpdate)
				},
				get 'Logger'() {
					return ___createMemoize___(this, 'Logger', () => BdApi.findModuleByProps('setLogFn')?.default)
				},
				get 'Navigation'() {
					return ___createMemoize___(this, 'Navigation', () => BdApi.findModuleByProps('replaceWith'))
				}
			},
			'@discord/components': {
				get 'Tooltip'() {
					return ___createMemoize___(this, 'Tooltip', () => BdApi.findModuleByDisplayName('Tooltip'))
				},
				get 'TooltipContainer'() {
					return ___createMemoize___(this, 'TooltipContainer', () => BdApi.findModuleByProps('TooltipContainer')?.TooltipContainer)
				},
				get 'TextInput'() {
					return ___createMemoize___(this, 'TextInput', () => BdApi.findModuleByDisplayName('TextInput'))
				},
				get 'SlideIn'() {
					return ___createMemoize___(this, 'SlideIn', () => BdApi.findModuleByDisplayName('SlideIn'))
				},
				get 'SettingsNotice'() {
					return ___createMemoize___(this, 'SettingsNotice', () => BdApi.findModuleByDisplayName('SettingsNotice'))
				},
				get 'TransitionGroup'() {
					return ___createMemoize___(this, 'TransitionGroup', () => BdApi.findModuleByDisplayName('TransitionGroup'))
				},
				get 'Button'() {
					return ___createMemoize___(this, 'Button', () => BdApi.findModuleByProps('DropdownSizes'))
				},
				get 'Flex'() {
					return ___createMemoize___(this, 'Flex', () => BdApi.findModuleByDisplayName('Flex'))
				},
				get 'Text'() {
					return ___createMemoize___(this, 'Text', () => BdApi.findModuleByDisplayName('Text'))
				},
				get 'Card'() {
					return ___createMemoize___(this, 'Card', () => BdApi.findModuleByDisplayName('Card'))
				}
			},
			'@discord/modules': {
				get 'Dispatcher'() {
					return ___createMemoize___(this, 'Dispatcher', () => BdApi.findModuleByProps('dirtyDispatch', 'subscribe'))
				},
				get 'EmojiUtils'() {
					return ___createMemoize___(this, 'EmojiUtils', () => BdApi.findModuleByProps('uploadEmoji'))
				},
				get 'PermissionUtils'() {
					return ___createMemoize___(this, 'PermissionUtils', () => BdApi.findModuleByProps('computePermissions'))
				},
				get 'DMUtils'() {
					return ___createMemoize___(this, 'DMUtils', () => BdApi.findModuleByProps('openPrivateChannel'))
				}
			},
			'@discord/stores': {
				get 'Messages'() {
					return ___createMemoize___(this, 'Messages', () => BdApi.findModuleByProps('getMessage', 'getMessages'))
				},
				get 'Channels'() {
					return ___createMemoize___(this, 'Channels', () => BdApi.findModuleByProps('getChannel'))
				},
				get 'Guilds'() {
					return ___createMemoize___(this, 'Guilds', () => BdApi.findModuleByProps('getGuild'))
				},
				get 'SelectedGuilds'() {
					return ___createMemoize___(this, 'SelectedGuilds', () => BdApi.findModuleByProps('getGuildId', 'getLastSelectedGuildId'))
				},
				get 'SelectedChannels'() {
					return ___createMemoize___(this, 'SelectedChannels', () => BdApi.findModuleByProps('getChannelId', 'getLastSelectedChannelId'))
				},
				get 'Info'() {
					return ___createMemoize___(this, 'Info', () => BdApi.findModuleByProps('getSessionId'))
				},
				get 'Status'() {
					return ___createMemoize___(this, 'Status', () => BdApi.findModuleByProps('getStatus'))
				},
				get 'Users'() {
					return ___createMemoize___(this, 'Users', () => BdApi.findModuleByProps('getUser', 'getCurrentUser'))
				},
				get 'SettingsStore'() {
					return ___createMemoize___(this, 'SettingsStore', () => BdApi.findModuleByProps('afkTimeout', 'status'))
				},
				get 'UserProfile'() {
					return ___createMemoize___(this, 'UserProfile', () => BdApi.findModuleByProps('getUserProfile'))
				},
				get 'Members'() {
					return ___createMemoize___(this, 'Members', () => BdApi.findModuleByProps('getMember'))
				},
				get 'Activities'() {
					return ___createMemoize___(this, 'Activities', () => BdApi.findModuleByProps('getActivities'))
				},
				get 'Games'() {
					return ___createMemoize___(this, 'Games', () => BdApi.findModuleByProps('getGame'))
				},
				get 'Auth'() {
					return ___createMemoize___(this, 'Auth', () => BdApi.findModuleByProps('getId', 'isGuest'))
				},
				get 'TypingUsers'() {
					return ___createMemoize___(this, 'TypingUsers', () => BdApi.findModuleByProps('isTyping'))
				}
			},
			'@discord/actions': {
				get 'ProfileActions'() {
					return ___createMemoize___(this, 'ProfileActions', () => BdApi.findModuleByProps('fetchProfile'))
				},
				get 'GuildActions'() {
					return ___createMemoize___(this, 'GuildActions', () => BdApi.findModuleByProps('requestMembersById'))
				}
			},
			get '@discord/i18n'() {
				return ___createMemoize___(this, '@discord/i18n', () => BdApi.findModuleByProps('getLocale'))
			},
			get '@discord/constants'() {
				return ___createMemoize___(this, '@discord/constants', () => BdApi.findModuleByProps('API_HOST'))
			},
			get '@discord/contextmenu'() {
				return ___createMemoize___(this, '@discord/contextmenu', () => {
					const ctx = Object.assign({}, BdApi.findModuleByProps('openContextMenu'), BdApi.findModuleByProps('MenuItem'));
					ctx.Menu = ctx.default;
					return ctx;
				})
			},
			get '@discord/forms'() {
				return ___createMemoize___(this, '@discord/forms', () => BdApi.findModuleByProps('FormItem'))
			},
			get '@discord/scrollbars'() {
				return ___createMemoize___(this, '@discord/scrollbars', () => BdApi.findModuleByProps('ScrollerAuto'))
			},
			get '@discord/native'() {
				return ___createMemoize___(this, '@discord/native', () => BdApi.findModuleByProps('requireModule'))
			},
			get '@discord/flux'() {
				return ___createMemoize___(this, '@discord/flux', () => Object.assign({}, BdApi.findModuleByProps('useStateFromStores').default, BdApi.findModuleByProps('useStateFromStores')))
			},
			get '@discord/modal'() {
				return ___createMemoize___(this, '@discord/modal', () => Object.assign({}, BdApi.findModuleByProps('ModalRoot'), BdApi.findModuleByProps('openModal')))
			},
			get '@discord/connections'() {
				return ___createMemoize___(this, '@discord/connections', () => BdApi.findModuleByProps('get', 'isSupported', 'map'))
			},
			get '@discord/sanitize'() {
				return ___createMemoize___(this, '@discord/sanitize', () => BdApi.findModuleByProps('stringify', 'parse', 'encode'))
			},
			get '@discord/icons'() {
				return ___createMemoize___(this, '@discord/icons', () => BdApi.findAllModules(m => m.displayName && ~m.toString().indexOf('currentColor')).reduce((icons, icon) => (icons[icon.displayName] = icon, icons), {}))
			},
			'@discord/classes': {
				get 'Timestamp'() {
					return ___createMemoize___(this, 'Timestamp', () => BdApi.findModuleByPrototypes('toDate', 'month'))
				},
				get 'Message'() {
					return ___createMemoize___(this, 'Message', () => BdApi.findModuleByPrototypes('getReaction', 'isSystemDM'))
				},
				get 'User'() {
					return ___createMemoize___(this, 'User', () => BdApi.findModuleByPrototypes('tag'))
				},
				get 'Channel'() {
					return ___createMemoize___(this, 'Channel', () => BdApi.findModuleByPrototypes('isOwner', 'isCategory'))
				}
			}
		};
		var __webpack_modules__ = {
			113: module => {
				module.exports = BdApi.React;
			}
		};
		var __webpack_module_cache__ = {};
		function __webpack_require__(moduleId) {
			var cachedModule = __webpack_module_cache__[moduleId];
			if (void 0 !== cachedModule) return cachedModule.exports;
			var module = __webpack_module_cache__[moduleId] = {
				exports: {}
			};
			__webpack_modules__[moduleId](module, module.exports, __webpack_require__);
			return module.exports;
		}
		(() => {
			__webpack_require__.d = (exports, definition) => {
				for (var key in definition)
					if (__webpack_require__.o(definition, key) && !__webpack_require__.o(exports, key)) Object.defineProperty(exports, key, {
						enumerable: true,
						get: definition[key]
					});
			};
		})();
		(() => {
			__webpack_require__.o = (obj, prop) => Object.prototype.hasOwnProperty.call(obj, prop);
		})();
		(() => {
			__webpack_require__.r = exports => {
				if ("undefined" !== typeof Symbol && Symbol.toStringTag) Object.defineProperty(exports, Symbol.toStringTag, {
					value: "Module"
				});
				Object.defineProperty(exports, "__esModule", {
					value: true
				});
			};
		})();
		var __webpack_exports__ = {};
		(() => {
			__webpack_require__.r(__webpack_exports__);
			__webpack_require__.d(__webpack_exports__, {
				default: () => Tuxphones
			});
			const external_fs_namespaceObject = require("fs");
			const external_net_namespaceObject = require("net");
			const external_path_namespaceObject = require("path");
			var React = __webpack_require__(113);
			const {
				Logger,
				Patcher,
				WebpackModules,
				DiscordModules,
				ContextMenu
			} = PluginApi;
			const {
				Dispatcher
			} = DiscordModules;
			const userMod = BdApi.findModuleByProps("getCurrentUser");
			const Tuxphones = class extends BasePlugin {
				onStart() {
					if (!process.env.HOME) {
						BdApi.showToast("XDG_RUNTIME_DIR is not defined.", {
							type: "error"
						});
						return;
					}
					this.sockPath = (0, external_path_namespaceObject.join)(process.env.HOME, ".config", "tuxphones.sock");
					this.serverSockPath = (0, external_path_namespaceObject.join)(process.env.HOME, ".config", "tuxphonesjs.sock");
					this.unixServer = (0, external_net_namespaceObject.createServer)((sock => {
						let data = [];
						sock.on("data", (d => data += d));
						sock.on("end", (() => {
							this.parseData(data);
							data = [];
						}));
					}));
					this.unixServer.listen(this.serverSockPath, (() => Logger.log("Server bound")));
					this.wsOnMessage = this.wsOnMessage.bind(this);
					this._onmessage = null;
					this._ws = null;
					Patcher.before(WebSocket.prototype, "send", ((that, [arg]) => {
						if ("string" !== typeof arg || !that.url.includes("discord") || this._ws && this._ws !== that) return;
						const json = JSON.parse(arg);
						console.log("%cWS SEND FRAME ================================", "color: green; font-size: large; margin-top: 20px;");
						if (0 === json.op && json.d.streams.length > 0 && "screen" === json.d.streams[0].type && json.d.user_id === userMod.getCurrentUser().id) {
							if (this._ws) this.resetVars();
							this._ws = that;
							this._onmessage = that.onmessage;
							that.onmessage = this.wsOnMessage;
						} else if (12 === json.op && 0 !== json.d.video_ssrc && 0 !== json.d.rtx_ssrc) {
							console.log("%cRECEIVED SSRC INFORMATION", "color: aqua; font-size: xx-large;");
							Logger.log("Video SSRC:");
							Logger.log(json.d.video_ssrc);
							Logger.log("RTX SSRC:");
							Logger.log(json.d.rtx_ssrc);
							this.ssrc = json.d.video_ssrc;
							const res = json.d.streams[0].max_resolution;
							this.resolution = {
								width: res.width,
								height: res.height,
								is_fixed: "fixed" === res.type
							};
						}
						Logger.log(json);
						console.log("%cWS END SEND FRAME ============================", "color: green; font-size: large; margin-bottom: 20px;");
					}));
					Patcher.before(WebSocket.prototype, "close", ((that, [arg]) => {
						Logger.log("CLOSE!");
						Logger.log(that);
						Logger.log(arg);
						if (this._ws === that) {
							console.log("%cSCREENSHARE CLOSED! Unlocking log...", "color: red; font-size: x-large;");
							if (this._ws) this.resetVars();
						}
					}));
					ContextMenu.getDiscordMenu("Confirm").then((m => {
						Patcher.after(m, "default", ((that, [arg], ret) => {
							Logger.log(that);
							Logger.log(arg);
							Logger.log(ret);
							if (!Array.isArray(ret.props.children)) return;
							if (arg.sound) ret.props.children[1] = React.createElement("p", {
								style: {
									color: "green",
									padding: "0px 16px"
								}
							}, "Tuxphones sound enabled!");
							else ret.props.children[1] = React.createElement("p", {
								style: {
									color: "red",
									padding: "0px 16px"
								}
							}, "Tuxphones not available.");
						}));
					}));
					new Promise((resolve => {
						const cancel = WebpackModules.addListener((module => {
							if (!module.default || !module.DesktopSources) return;
							resolve(module);
							cancel();
						}));
					})).then((m => {
						Patcher.after(m, "default", ((_, __, ret) => ret.then((vals => new Promise((res => {
							Dispatcher.subscribe("TUX_APPS", (function dispatch(e) {
								Dispatcher.unsubscribe("TUX_APPS", dispatch);
								res(vals.map((v => {
									v.sound = v.id.startsWith("window") && e.apps.includes(v.id.split(":")[1]);
									return v;
								})));
							}));
							this.getInfo();
						}))))));
					}));
					this.getInfo();
				}
				resetVars() {
					this._ws.onmessage = this._onmessage;
					this._ws = null;
					this._onmessage = null;
				}
				wsOnMessage(m) {
					this._onmessage(m);
					const json = JSON.parse(m.data);
					console.log("%cWS RECV FRAME ================================", "color: orange; font-size: large; margin-top: 20px;");
					if (4 === json.op) {
						console.log("%cRECEIVED CODEC AND ENCRYPTION INFORMATION", "color: aqua; font-size: xx-large;");
						Logger.log("Audio Codec:");
						Logger.log(json.d.audio_codec);
						Logger.log("Encryption Mode:");
						Logger.log(json.d.mode);
						Logger.log("Secret key:");
						Logger.log(json.d.secret_key);
					}
					Logger.log(json);
					console.log("%cWS END RECV FRAME ============================", "color: orange; font-size: large; margin-bottom: 20px;");
				}
				parseData(data) {
					let obj = JSON.parse(data);
					Logger.log(obj);
					switch (obj.type) {
						case "ApplicationList":
							const {
								apps
							} = obj;
							break;
						case "ConnectionId":
							const {
								id
							} = obj;
							break;
						default:
							Logger.err(`Received unknown command type: ${obj.type}`);
					}
				}
				startStream(ip, port, key, pid, resolution, ssrc) {
					this.unixClient = (0, external_net_namespaceObject.createConnection)(this.sockPath, (() => {
						this.unixClient.write(JSON.stringify({
							type: "StartStream",
							ip,
							port,
							key,
							pid,
							resolution,
							ssrc
						}));
						this.unixClient.destroy();
					}));
				}
				endStream() {
					this.unixClient = (0, external_net_namespaceObject.createConnection)(this.sockPath, (() => {
						this.unixClient.write(JSON.stringify({
							type: "StopStream"
						}));
						this.unixClient.destroy();
					}));
				}
				getInfo() {
					Dispatcher.dirtyDispatch({
						type: "TUX_APPS",
						apps: []
					});
					this.unixClient = (0, external_net_namespaceObject.createConnection)(this.sockPath, (() => {
						this.unixClient.write(JSON.stringify({
							type: "GetInfo"
						}));
						this.unixClient.destroy();
					}));
				}
				onStop() {
					if (this.unixServer && this.unixServer.listening) this.unixServer.close();
					if ((0, external_fs_namespaceObject.existsSync)(this.serverSockPath))(0, external_fs_namespaceObject.unlinkSync)(this.serverSockPath);
					if (this._ws) this.resetVars();
				}
			};
		})();
		module.exports.LibraryPluginHack = __webpack_exports__;
	})();
	const PluginExports = module.exports.LibraryPluginHack;
	return PluginExports?.__esModule ? PluginExports.default : PluginExports;
}
module.exports = window.hasOwnProperty("ZeresPluginLibrary") ?
	buildPlugin(window.ZeresPluginLibrary.buildPlugin(config)) :
	class {
		getName() {
			return config.info.name;
		}
		getAuthor() {
			return config.info.authors.map(a => a.name).join(", ");
		}
		getDescription() {
			return `${config.info.description}. __**ZeresPluginLibrary was not found! This plugin will not work!**__`;
		}
		getVersion() {
			return config.info.version;
		}
		load() {
			BdApi.showConfirmationModal(
				"Library plugin is needed",
				[`The library plugin needed for ${config.info.name} is missing. Please click Download to install it.`], {
					confirmText: "Download",
					cancelText: "Cancel",
					onConfirm: () => {
						require("request").get("https://rauenzi.github.io/BDPluginLibrary/release/0PluginLibrary.plugin.js", async (error, response, body) => {
							if (error) return require("electron").shell.openExternal("https://betterdiscord.net/ghdl?url=https://raw.githubusercontent.com/rauenzi/BDPluginLibrary/master/release/0PluginLibrary.plugin.js");
							await new Promise(r => require("fs").writeFile(require("path").join(BdApi.Plugins.folder, "0PluginLibrary.plugin.js"), body, r));
						});
					}
				}
			);
		}
		start() {}
		stop() {}
	};
/*@end@*/