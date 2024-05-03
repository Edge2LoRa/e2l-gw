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
        pub timestamp: u32,
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
                timestamp: 0,
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
        pub fcnt: u16,
        pub timestamp: u32,
        pub frequency: f32,
        pub data_rate: String,
        pub coding_rate: String,
        pub gtw_id: String,
        pub gtw_channel: u32,
        pub gtw_rssi: i32,
        pub gtw_snr: f32,
        pub encrypted_payload: String,
    }
    impl Default for UnassociatedMqttJson {
        fn default() -> Self {
            UnassociatedMqttJson {
                dev_eui: "".to_string(),
                dev_addr: "".to_string(),
                gw_id: "".to_string(),
                fcnt: 0,
                timestamp: 0,
                frequency: 868.1,
                data_rate: "SF7BW125".to_string(),
                coding_rate: "4/5".to_string(),
                gtw_id: "".to_string(),
                gtw_channel: 0,
                gtw_rssi: 0,
                gtw_snr: 0.0,
                encrypted_payload: "".to_string(),
            }
        }
    }
    // #[derive(Debug, Serialize, Deserialize)]
    // pub struct MqttJson {
    //     pub index: u64,
    //     pub gwmac: String,
    //     pub deveui: String,
    //     pub devaddr: String,
    //     pub fcnt: u32,
    //     pub ftype: String,
    //     pub rssi: i32,
    //     pub lsnr: f32,
    //     pub size: u32,
    //     pub chan: u32,
    //     pub freq: f32,
    //     pub datr: String,
    //     pub tmst: u64,
    //     pub time: String,
    //     pub agent_time: u64,
    //     pub end_line: u64,
    // }
    // impl Default for MqttJson {
    //     fn default() -> Self {
    //         MqttJson {
    //             index: rand::thread_rng().gen_range(0, 18446744073709551615),
    //             gwmac: "0000000000000000".to_string(),
    //             deveui: "0000000000000000".to_string(),
    //             devaddr: "00000000".to_string(),
    //             fcnt: 0,
    //             ftype: "Missing".to_string(),
    //             rssi: 0,
    //             lsnr: 0.0,
    //             size: 0,
    //             chan: 0,
    //             freq: 0.0,
    //             datr: "Missing".to_string(),
    //             tmst: 0,
    //             time: "2000-01-01T00:00:00.000000Z".to_string(),
    //             agent_time: SystemTime::now()
    //                 .duration_since(SystemTime::UNIX_EPOCH)
    //                 .unwrap()
    //                 .as_secs()
    //                 * 1000,
    //             end_line: 4753416825896106269,
    //         }
    //     }
    // }
}
