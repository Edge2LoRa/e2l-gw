pub(crate) mod mqtt_structs {
    use serde_derive::Deserialize;
    use serde_derive::Serialize;

    #[derive(Debug, Serialize, Deserialize)]
    pub struct MqttVariables {
        pub broker_url: String,
        pub broker_port: String,
        pub broker_auth_name: String,
        pub broker_auth_password: String,
        pub broker_process_topic: String,
        pub broker_handover_topic: String,
        pub broker_qos: i32,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct MqttJson {
        pub dev_eui: String,
        pub dev_addr: String,
        pub fcnt: u16,
        pub timestamp: String,
        pub frequency: f32,
        pub data_rate: String,
        pub coding_rate: String,
        pub gtw_id: String,
        pub gtw_channel: u32,
        pub gtw_rssi: i32,
        pub gtw_snr: f32,
        pub payload: String,
    }
    impl Default for MqttJson {
        fn default() -> Self {
            MqttJson {
                dev_eui: "".to_string(),
                dev_addr: "".to_string(),
                fcnt: 0,
                timestamp: "".to_string(),
                frequency: 868.1,
                data_rate: "SF7BW125".to_string(),
                coding_rate: "4/5".to_string(),
                gtw_id: "".to_string(),
                gtw_channel: 0,
                gtw_rssi: 0,
                gtw_snr: 0.0,
                payload: "".to_string(),
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct UnassociatedMqttJson {
        pub dev_eui: String,
        pub dev_addr: String,
        pub gw_id: String,
        pub gwmac: String,
        pub fcnt: u16,
        // RxpkContent
        pub time: Option<String>,
        pub tmst: u32,
        pub freq: f32,
        pub chan: Option<u32>,
        pub stat: Option<i32>,
        pub modu: String,
        pub datr: String,
        pub codr: String,
        pub rssi: Option<i32>,
        pub lsnr: Option<f32>,
        pub size: u32,
        pub data: String,
    }
    // impl Default for UnassociatedMqttJson {
    //     fn default() -> Self {
    //         UnassociatedMqttJson {
    //             dev_eui: "".to_string(),
    //             dev_addr: "".to_string(),
    //             gw_id: "".to_string(),
    //             fcnt: 0,
    //             timestamp: 0,
    //             frequency: 868.1,
    //             data_rate: "SF7BW125".to_string(),
    //             coding_rate: "4/5".to_string(),
    //             gtw_id: "".to_string(),
    //             gtw_channel: 0,
    //             gtw_rssi: 0,
    //             gtw_snr: 0.0,
    //             encrypted_payload: "".to_string(),
    //         }
    //     }
    // }
}
