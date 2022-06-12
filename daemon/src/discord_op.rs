pub mod opcodes {
    use crate::EncryptionAlgorithm;

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct WebsocketMessage {
        pub op: u8,
        // TODO: find a way to tell serde to deserialize this based on the op
        pub d: WebsocketMessageD,
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    #[serde(untagged)]
    pub enum WebsocketMessageD {
        /// auth
        OpCode0(OpCode0),
        /// Outgoing message containing info about the stream
        OpCode1(OpCode1),
        /// Incoming message containing configuration options for webrtc connection
        OpCode2(OpCode2),
        /// Outgoind heartbeat message
        OpCode3(OpCode3_6),
        /// Incoming heartbeat message
        OpCode6(OpCode3_6),
        /// Initial heartbeat incoming configuration message
        OpCode8(OpCode8),
        /// Outgoing message containing info about the stream
        OpCode12(OpCode12),
        /// Unknown outgoing message
        OpCode15(OpCode15),
    }


    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct GatewayResolution {
        #[serde(rename = "type")]
        resolution_type: String,
        width: u16,
        height: u16,
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct GatewayStream {
        //Opcode 0 and 2 and 12 params
        #[serde(rename = "type")]
        pub stream_type: String,
        pub rid: u8,
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
    #[derive(serde::Serialize, serde::Deserialize, Debug)]
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
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct OpCode1Data {
        /// My public ip address obtainable with an UDP IP discovery message
        pub address: String,
        /// Encryption algorithm to use
        pub mode: EncryptionAlgorithm,
        /// My public port obtainable with an UDP IP discovery message
        port: u16,
    }

    /// Outgoing message containing info about the stream
    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct OpCode1 {
        /// My public ip address obtainable with an UDP IP discovery message
        address: String,
        /// My public port obtainable with an UDP IP discovery message
        port: u16,
        experiments: Vec<String>,
        /// Encryption algorithm to use
        mode: EncryptionAlgorithm,
        protocol: String,
        rtc_connection_id: String,
        codecs: Vec<GatewayCodec>,
        data: OpCode1Data,
    }

    /// Incoming message containing configuration options for webrtc connection
    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct OpCode2 {
        experiment: Vec<String>,
        /// Discord ip address to stream to
        ip: String,
        /// Discord port to stream to
        port: u16,
        /// Supported encrpytion modes by the server
        modes: Vec<String>,
        ssrc: u32,
        streams: Vec<GatewayStream>,
    }

    /// Heartbeat message
    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct OpCode3_6 {
        /// Random nonce
        d: u64,
    }

    /// Initial heartbeat incoming configuration message
    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct OpCode8 {
        /// the interval (in milliseconds) the client should heartbeat with
        heartbeat_interval: u32,
        /// api version
        v: u8,
    }

    /// Outgoing message containing info about the stream
    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct OpCode12 {
        audio_ssrc: u32,
        rtx_ssrc: u32,
        video_ssrc: u32,
        streams: Vec<GatewayStream>,
    }

    ///  Unknown outgoing message
    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct OpCode15 {
        any: u8,
    }
}