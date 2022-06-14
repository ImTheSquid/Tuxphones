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
		var __webpack_require__ = {};
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
		__webpack_require__.r(__webpack_exports__);
		__webpack_require__.d(__webpack_exports__, {
			default: () => Tuxphones
		});
		const external_fs_namespaceObject = require("fs");
		const external_net_namespaceObject = require("net");
		const external_path_namespaceObject = require("path");
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
		const React = BdApi.React;
		const AuthenticationStore = BdApi.findModule((m => m.default.getToken)).default;
		const RTCConnectionStore = BdApi.findModule((m => m.default.getRTCConnectionId && m.default._changeCallbacks.size)).default;
		const UserStatusStore = BdApi.findModule((m => m.default.getVoiceChannelId)).default;
		const WebSocketControl = BdApi.findModuleByPrototypes("streamCreate");
		const Button = BdApi.findModuleByProps("BorderColors");
		const Tuxphones = class extends BasePlugin {
			onStart() {
				if (!process.env.HOME) {
					BdApi.showToast("$HOME is not defined. Reload Discord after defining.", {
						type: "error"
					});
					throw "$HOME is not defined.";
				}
				this.sockPath = (0, external_path_namespaceObject.join)(process.env.HOME, ".config", "tuxphones.sock");
				if (!(0, external_fs_namespaceObject.existsSync)(this.sockPath)) {
					BdApi.showConfirmationModal("Tuxphones Daemon Error", ["The Tuxphones daemon was not detected.\n", "If you don't know what this means or installed just the plugin and not the daemon, get help installing the daemon by going to the GitHub page:", React.createElement("a", {
						href: "https://github.com/ImTheSquid/Tuxphones",
						target: "_blank"
					}, "Tuxphones Github"), " \n", "If you're sure you already installed the daemon, make sure it's running then click \"Reload Discord\"."], {
						danger: true,
						confirmText: "Reload Discord",
						cancelText: "Stop Tuxphones",
						onConfirm: () => {
							location.reload();
						}
					});
					throw "Daemon not running!";
				}
				this.serverSockPath = (0, external_path_namespaceObject.join)(process.env.HOME, ".config", "tuxphonesjs.sock");
				if ((0, external_fs_namespaceObject.existsSync)(this.serverSockPath))(0, external_fs_namespaceObject.unlinkSync)(this.serverSockPath);
				this.unixServer = (0, external_net_namespaceObject.createServer)((sock => {
					let data = [];
					sock.on("data", (d => data += d));
					sock.on("end", (() => {
						this.parseData(data);
						data = [];
					}));
				}));
				this.unixServer.listen(this.serverSockPath, (() => Logger.log("Server bound")));
				this.interceptNextStreamServerUpdate = false;
				this.currentSoundProfile = null;
				this.selectedFPS = null;
				this.selectedResoultion = null;
				this.serverId = null;
				this.webSocketControlObj = null;
				Patcher.before(WebSocketControl.prototype, "_handleDispatch", (that => {
					this.webSocketControlObj = that;
				}));
				Patcher.instead(Dispatcher, "dispatch", ((_, [arg], original) => {
					if (this.interceptNextStreamServerUpdate && "STREAM_SERVER_UPDATE" === arg.type) {
						let res = null;
						switch (this.selectedResoultion) {
							case 720:
								res = {
									width: 1280,
									height: 720,
									is_fixed: true
								};
								break;
							case 1080:
								res = {
									width: 1920,
									height: 1080,
									is_fixed: true
								};
								break;
							default:
								res = {
									width: 0,
									height: 0,
									is_fixed: false
								};
								break;
						}
						this.startStream(this.currentSoundProfile.pid, this.currentSoundProfile.xid, res, this.selectedFPS, this.serverId, arg.token, arg.endpoint);
						return;
					}
					original(arg);
				}));
				ContextMenu.getDiscordMenu("GoLiveModal").then((m => {
					Patcher.after(m, "default", ((_, __, ret) => {
						Logger.log(ret);
						if (2 == ret.props.children.props.children[2].props.children[1].props.activeSlide && ret.props.children.props.children[2].props.children[1].props.children[2].props.children.props.children.props.selectedSource?.sound) ret.props.children.props.children[2].props.children[2].props.children[0] = React.createElement("div", {
							style: {
								"margin-right": "8px"
							}
						}, React.createElement(Button, {
							onClick: () => {
								const streamInfo = ret.props.children.props.children[2].props.children[1].props.children[2].props.children.props.children.props;
								this.currentSoundProfile = streamInfo.selectedSource.sound;
								this.selectedFPS = streamInfo.selectedFPS;
								this.selectedResoultion = streamInfo.selectedResoultion;
								this.serverId = streamInfo.guildId;
								this.createStream(streamInfo.guildId, UserStatusStore.getVoiceChannelId());
							},
							size: Button.Sizes.SMALL
						}, "Go Live with Sound"));
					}));
				}));
				ContextMenu.getDiscordMenu("Confirm").then((m => {
					Patcher.after(m, "default", ((_, [arg], ret) => {
						if (!Array.isArray(ret.props.children)) return;
						Logger.log(arg);
						if (arg.selectedSource.sound) ret.props.children[1] = React.createElement("p", {
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
							Logger.log(vals);
							Logger.log(e.apps);
							res(vals.map((v => {
								let found = e.apps.find((el => el.xid == v.id.split(":")[1]));
								if (v.id.startsWith("window") && found) v.sound = found;
								else v.sound = null;
								return v;
							})));
						}));
						this.getInfo(vals.filter((v => v.id.startsWith("window"))).map((v => parseInt(v.id.split(":")[1]))));
					}))))));
				}));
			}
			createStream(guild_id, channel_id) {
				this.interceptNextStreamServerUpdate = true;
				this.webSocketControlObj.streamCreate(null === guild_id ? "call" : "guild", guild_id, channel_id, null);
			}
			parseData(data) {
				let obj = JSON.parse(data);
				Logger.log(obj);
				switch (obj.type) {
					case "ApplicationList":
						Dispatcher.dirtyDispatch({
							type: "TUX_APPS",
							apps: obj.apps
						});
						break;
					default:
						Logger.err(`Received unknown command type: ${obj.type}`);
				}
			}
			startStream(pid, xid, resolution, framerate, server_id, token, endpoint) {
				this.unixClient = (0, external_net_namespaceObject.createConnection)(this.sockPath, (() => {
					this.unixClient.write(JSON.stringify({
						type: "StartStream",
						pid,
						xid,
						resolution,
						framerate,
						server_id,
						user_id: AuthenticationStore.getId(),
						token,
						session_id: AuthenticationStore.getSessionId(),
						rtc_connection_id: RTCConnectionStore.getRTCConnectionId(),
						endpoint
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
			getInfo(xids) {
				this.unixClient = (0, external_net_namespaceObject.createConnection)(this.sockPath, (() => {
					this.unixClient.write(JSON.stringify({
						type: "GetInfo",
						xids
					}));
					this.unixClient.destroy();
				}));
				this.unixClient.on("error", (e => {
					Logger.err(`[GetInfo] Socket client error: ${e}`);
					Dispatcher.dirtyDispatch({
						type: "TUX_APPS",
						apps: []
					});
				}));
			}
			onStop() {
				if (this.unixServer && this.unixServer.listening) this.unixServer.close();
				if ((0, external_fs_namespaceObject.existsSync)(this.serverSockPath))(0, external_fs_namespaceObject.unlinkSync)(this.serverSockPath);
				Patcher.unpatchAll();
			}
		};
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