pub mod opcodes {
    use lazy_static::lazy_static;
    use regex::Regex;
    use crate::{EncryptionAlgorithm, receive::StreamResolutionInformation};

    lazy_static! {
        static ref OPCODE_OUTGOING_REGEX: Regex = Regex::new(r#""op":"(?P<op>\d+)""#).unwrap();
    }


    #[derive(serde::Serialize, Debug)]
    #[serde(tag = "op", content = "d")]
    pub enum OutgoingWebsocketMessage {
        /// auth
        #[serde(rename = "0")]
        OpCode0(OpCode0),
        /// Message containing info about the stream
        #[serde(rename = "1")]
        OpCode1(OpCode1),
        /// Heartbeat message
        #[serde(rename = "3")]
        OpCode3(OpCode3_6),
        /// Message containing info about the stream
        #[serde(rename = "12")]
        OpCode12(OpCode12),
        /// Unknown message
        #[serde(rename = "15")]
        OpCode15(OpCode15),
    }

    impl OutgoingWebsocketMessage {
        pub fn to_json(&self) -> String {
            OPCODE_OUTGOING_REGEX.replace(&serde_json::to_string(&self).unwrap(), "\"op\":$op").to_string()
        }
    }

    #[derive(serde::Deserialize, Debug)]
    #[serde(tag = "op", content = "d")]
    pub enum IncomingWebsocketMessage {
        /// Message containing configuration options for webrtc connection
        #[serde(rename = "2")]
        OpCode2(OpCode2),
        /// Encryption information
        #[serde(rename = "4")]
        OpCode4(OpCode4),
        /// Heartbeat message
        #[serde(rename = "6")]
        OpCode6(OpCode3_6),
        /// Initial heartbeat incoming configuration message
        #[serde(rename = "8")]
        OpCode8(OpCode8),
    }


    #[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
    pub struct GatewayResolution {
        #[serde(rename = "type")]
        pub resolution_type: String,
        pub width: u16,
        pub height: u16,
    }

    impl GatewayResolution {
        pub fn from_socket_info(info: StreamResolutionInformation) -> Self {
            GatewayResolution { resolution_type: if info.is_fixed {
                "fixed"
            } else {
                "source"
            }.to_string(), width: info.width, height: info.height }
        }
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct GatewayStream {
        //Opcode 0 and 2 and 12 params
        #[serde(rename = "type")]
        pub stream_type: String,
        pub rid: String,
        pub quality: u8,

        //Opcode 2 and 12 params
        #[serde(skip_serializing_if = "Option::is_none")]
        pub active: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub ssrc: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub rtx_ssrc: Option<u32>,

        //Opcode 12 params
        #[serde(skip_serializing_if = "Option::is_none")]
        pub max_bitrate: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub max_framerate: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub max_resolution: Option<GatewayResolution>,
    }

    /// Outgoing message containing info about the stream
    #[derive(serde::Serialize, Debug)]
    pub struct OpCode0 {
        pub server_id: String,
        pub session_id: String,
        pub streams: Vec<GatewayStream>,
        /// session token
        pub token: String,
        pub user_id: String,
        pub video: bool,
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub enum PayloadType {
        #[serde(rename = "audio")]
        Audio,
        #[serde(rename = "video")]
        Video,
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct GatewayCodec {
        pub name: String,
        #[serde(rename = "type")]
        pub codec_type: PayloadType,
        pub priority: u16,
        pub payload_type: u8,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub rtx_payload_type: Option<u8>
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct OpCode1Data {
        /// My public ip address obtainable with an UDP IP discovery message
        pub address: String,
        /// Encryption algorithm to use
        pub mode: EncryptionAlgorithm,
        /// My public port obtainable with an UDP IP discovery message
        pub port: u16,
    }

    /// Outgoing message containing info about the stream
    #[derive(serde::Serialize, Debug)]
    pub struct OpCode1 {
        /// My public ip address obtainable with an UDP IP discovery message
        pub address: String,
        /// My public port obtainable with an UDP IP discovery message
        pub port: u16,
        pub experiments: Vec<String>,
        /// Encryption algorithm to use
        pub mode: EncryptionAlgorithm,
        pub protocol: String,
        pub rtc_connection_id: String,
        pub codecs: Vec<GatewayCodec>,
        pub data: OpCode1Data,
    }

    /// Incoming message containing configuration options for webrtc connection
    #[derive(serde::Deserialize, Debug)]
    pub struct OpCode2 {
        pub experiment: Vec<String>,
        /// Discord ip address to stream to
        pub ip: String,
        /// Discord port to stream to
        pub port: u16,
        /// Supported encrpytion modes by the server
        pub modes: Vec<String>,
        pub ssrc: u32,
        pub streams: Vec<GatewayStream>,
    }

    /// Heartbeat message
    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct OpCode3_6 {
        /// Random nonce
        pub d: u64,
    }

    #[derive(serde::Deserialize, Debug)]
    pub enum AudioCodec {
        #[serde(rename = "opus")]
        Opus
    }

    #[derive(serde::Deserialize, Debug)]
    pub struct OpCode4 {
        /// Audio codec
        pub audio_codec: AudioCodec,
        /// Unknown value
        pub media_session_id: String,
        pub encryption_mode: String,
        pub secret_key: Vec<u8>,
        pub video_codec: String
    }

    /// Initial heartbeat incoming configuration message
    #[derive(serde::Deserialize, Debug)]
    pub struct OpCode8 {
        /// the interval (in milliseconds) the client should heartbeat with
        pub heartbeat_interval: u64,
        /// api version
        pub v: u8,
    }

    /// Outgoing message containing info about the stream
    #[derive(serde::Serialize, Debug)]
    pub struct OpCode12 {
        pub audio_ssrc: u32,
        pub rtx_ssrc: u32,
        pub video_ssrc: u32,
        pub streams: Vec<GatewayStream>,
    }

    ///  Unknown outgoing message
    #[derive(serde::Serialize, Debug)]
    pub struct OpCode15 {
        pub any: u8,
    }
}