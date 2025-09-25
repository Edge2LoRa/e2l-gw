pub(crate) mod e2l_mqtt_client {

    use futures::{executor::block_on, stream::StreamExt};
    use serde_derive::Deserialize;
    use serde_derive::Serialize;
    use serde_json::Error;
    use std::collections::HashSet;
    use std::env;
    use std::sync::{Arc, Mutex};

    use crate::e2l_crypto::e2l_crypto::E2LCrypto;
    // use crate::lorawan_structs::lora_structs::{Rxpk, RxpkContent};
    use crate::e2l_end_device::e2l_end_device::{DeviceStats};
    use paho_mqtt as mqtt;
    use reqwest::Client;
    use std::time::Duration;
    use std::collections::HashMap;
    
    // One Global variable will be used by multiple threads
    use once_cell::sync::Lazy;

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct FrameCounters {
        pub rx_frames: u32,//The number of LoRaWAN frames that has received by the gateway from the end-device(Uplink messages)
        pub fw_frames: u32,//--> The number of LoRaWAN frames transmitted by the GW to the Network Server using the standard LoRaWAN Specification during the reporting period.
        pub tx_ho_frames: u32,//--> The number of frames the GW forwarded to other GWs using the handover procedure during the reporting period.
        pub rx_ho_frames: u32,//--> The number of frames  received by the GW by the other GWs using the handover procedure
        pub proc_frames: u32,//--> The number of frames the GW locally processed, i.e. the number of frames the Parser Module published to the process topic.
    }
    impl Default for FrameCounters {
        fn default() -> Self {
            FrameCounters {
                rx_frames: 0,
                fw_frames: 0,
                tx_ho_frames: 0,
                rx_ho_frames: 0,
                proc_frames: 0,
            }
        }
    }
    // A thread-safe, global mutable instance
    pub static FRAME_COUNTERS: Lazy<Mutex<FrameCounters>> = Lazy::new(|| {
        Mutex::new(FrameCounters::default())
    });
    #[derive(Debug, Serialize, Deserialize)]
    pub struct MqttVariables {
        pub broker_url: String,
        pub broker_port: String,
        pub broker_api_port: String,
        pub broker_auth_name: String,
        pub broker_auth_password: String,
        pub broker_process_topic: String,
        pub broker_handover_topic: String,
        pub broker_control_topic: String,
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
        pub rx_gw: String,
        pub process_gw: String,
        pub gtw_channel: u32,
        pub gtw_rssi: i32,
        pub gtw_snr: f32,
        pub payload: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct UnassociatedMqttJson {
        pub dev_eui: String,
        pub dev_addr: String,
        pub gw_id: String,
        pub gwmac: String,
        pub fcnt: u16,
        pub rx_gw: String,
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

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct GwStats {
        pub gw_id: String,
        pub frame: FrameCounters,
        pub mem_available: u64,
        pub mem_usage: u64,
        pub mem_usage_percentage: u64,
        pub swp_usage_percentage:u64,
        pub ntwk_down:i32,
        pub ntwk_up:i32,
        pub cpu_usage: f32,
        pub cpu_usage_percentage:f32,
    }
    #[derive(Debug, Serialize)]
    pub struct CombinedStats{
        pub gw_stats: GwStats,
        pub devices_stats: HashMap<String, DeviceStats>
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct NewAssignedDevice {
        pub dev_eui: String,
        pub dev_addr: String,
        pub edge_s_enc_key: String,
        pub edge_s_int_key: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct NewUnassociatedDevice {
        pub dev_eui: String,
        pub dev_addr: String,
        pub assigned_gw: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct RemoveDevice {
        pub dev_addr: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct GwActiveStatus {
        pub active: bool,
    }

    pub struct E2LMqttClient {
        gw_id: String,
        mqtt_client: mqtt::AsyncClient,
        mqtt_process_topic: String,
        mqtt_handover_base_topic: String,
        mqtt_control_topic: String,
        mqtt_qos: i32,
        e2l_crypto: Arc<Mutex<E2LCrypto>>,
        api_endpoint: String,
        api_username: String,
        api_password: String,
    }

    /*
       CRYPTO CONFIGURATION
    */

    #[derive(Debug, Serialize, Deserialize)]
    pub struct GWPubInfo {
        pub mqtt_port: String,
        pub pub_key: Vec<u8>,
    }

    /*
       MQTT BRIDGE CONFIGURATION
    */
    #[derive(Debug, Serialize, Deserialize)]
    pub struct MqttBridgeResourceOpts {
        max_buffer_size: u32,
        query_mode: String,
        health_check_interval: String,
    }
    impl Default for MqttBridgeResourceOpts {
        fn default() -> Self {
            MqttBridgeResourceOpts {
                max_buffer_size: 104857600,
                query_mode: "sync".to_string(),
                health_check_interval: "15s".to_string(),
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct MqqtBridgeSslConfig {
        pub enable: bool,
    }
    impl Default for MqqtBridgeSslConfig {
        fn default() -> Self {
            MqqtBridgeSslConfig { enable: false }
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct MqttBridgeRemoteOpts {
        pub topic: String,
        pub qos: String,
        pub retain: String,
        pub payload: String,
    }
    impl MqttBridgeRemoteOpts {
        fn new(topic: String) -> Self {
            MqttBridgeRemoteOpts {
                topic: topic,
                qos: "${qos}".to_string(),
                retain: "${retain}".to_string(),
                payload: "${payload}".to_string(),
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct MqttBridgeLocalOpts {
        pub topic: String,
    }
    impl MqttBridgeLocalOpts {
        fn new(topic: String) -> Self {
            MqttBridgeLocalOpts { topic: topic }
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct MqttBridgeEgressOpts {
        pool_size: u32,
        remote: MqttBridgeRemoteOpts,
        local: MqttBridgeLocalOpts,
    }
    impl MqttBridgeEgressOpts {
        fn new(local_topic: String, remote_topic: String) -> Self {
            MqttBridgeEgressOpts {
                pool_size: 8,
                remote: MqttBridgeRemoteOpts::new(remote_topic),
                local: MqttBridgeLocalOpts::new(local_topic),
            }
        }
    }
    #[derive(Debug, Serialize, Deserialize)]
    pub struct MqttBridgeIngressOpts {
        pool_size: u32,
        local: MqttBridgeRemoteOpts,
        remote: MqttBridgeLocalOpts,
    }
    impl MqttBridgeIngressOpts {
        fn _new(local_topic: String, remote_topic: String) -> Self {
            MqttBridgeIngressOpts {
                pool_size: 8,
                local: MqttBridgeRemoteOpts::new(local_topic),
                remote: MqttBridgeLocalOpts::new(remote_topic),
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct MqttBridgeEgressConfig {
        pub name: String,
        pub r#type: String,
        pub enable: bool,
        pub bridge_mode: bool,
        pub resource_opts: MqttBridgeResourceOpts,
        pub server: String,
        pub proto_ver: String,
        pub username: String,
        pub password: String,
        pub ssl: MqqtBridgeSslConfig,
        pub egress: MqttBridgeEgressOpts,
    }
    impl MqttBridgeEgressConfig {
        fn new(
            name: String,
            server: String,
            username: String,
            password: String,
            local_topic: String,
            remote_topic: String,
        ) -> Self {
            MqttBridgeEgressConfig {
                name: name,
                r#type: "mqtt".to_string(),
                enable: true,
                bridge_mode: true,
                resource_opts: MqttBridgeResourceOpts::default(),
                server: server,
                proto_ver: "v5".to_string(),
                username: username,
                password: password,
                ssl: MqqtBridgeSslConfig::default(),
                egress: MqttBridgeEgressOpts::new(local_topic, remote_topic),
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct MqttBridgeIngressConfig {
        pub name: String,
        pub r#type: String,
        pub enable: bool,
        pub resource_opts: MqttBridgeResourceOpts,
        pub server: String,
        pub proto_ver: String,
        pub username: String,
        pub password: String,
        pub ssl: MqqtBridgeSslConfig,
        pub ingress: MqttBridgeIngressOpts,
    }
    impl MqttBridgeIngressConfig {
        fn _new(
            name: String,
            server: String,
            username: String,
            password: String,
            local_topic: String,
            remote_topic: String,
        ) -> Self {
            MqttBridgeIngressConfig {
                name: name,
                r#type: "mqtt".to_string(),
                enable: true,
                resource_opts: MqttBridgeResourceOpts::default(),
                server: server,
                proto_ver: "v5".to_string(),
                username: username,
                password: password,
                ssl: MqqtBridgeSslConfig::default(),
                ingress: MqttBridgeIngressOpts::_new(local_topic, remote_topic),
            }
        }
    }

    impl E2LMqttClient {
        pub fn new(
            gw_id: String,
            client_id: String,
            mqtt_variables: MqttVariables,
            e2l_crypto: Arc<Mutex<E2LCrypto>>,
        ) -> Self {
            let host = format!(
                "{}:{}",
                mqtt_variables.broker_url, mqtt_variables.broker_port
            );
            let create_opts = mqtt::CreateOptionsBuilder::new()
                .server_uri(host)
                .client_id(client_id)
                .finalize();
            let mqtt_client: mqtt::AsyncClient = mqtt::AsyncClient::new(create_opts)
                .unwrap_or_else(|err| {
                    println!("Error creating the client: {:?}", err);
                    std::process::exit(1);
                });

            // Connection options
            let mut mqtt_conn_opts_builder: mqtt::ConnectOptionsBuilder =
                mqtt::ConnectOptionsBuilder::new_v5();
            mqtt_conn_opts_builder.user_name(mqtt_variables.broker_auth_name.clone());
            mqtt_conn_opts_builder.password(mqtt_variables.broker_auth_password.clone());
            let connect_result = mqtt_client
                .connect(mqtt_conn_opts_builder.finalize())
                .wait();
            if let Err(e) = connect_result {
                println!("Error connecting to the broker: {:?}", e);
                std::process::exit(1);
            }

            // Subscribe to HANDOVER TOPIC
            let handover_base_topic = mqtt_variables.broker_handover_topic.clone();

            /*
               API ENDPOINT
            */
            let api_endpoint = format!(
                "http://{}:{}/api/v5/",
                mqtt_variables.broker_url, mqtt_variables.broker_api_port
            );
            let api_username = env::var("BROKER_API_USERNAME").unwrap();
            let api_password = env::var("BROKER_API_PASSWORD").unwrap();

            E2LMqttClient {
                gw_id: gw_id,
                mqtt_client: mqtt_client,
                mqtt_process_topic: mqtt_variables.broker_process_topic,
                mqtt_handover_base_topic: handover_base_topic,
                mqtt_control_topic: mqtt_variables.broker_control_topic,
                mqtt_qos: mqtt_variables.broker_qos,
                api_endpoint: api_endpoint,
                api_username: api_username,
                api_password: api_password,
                e2l_crypto: e2l_crypto,
            }
        }

        pub fn publish_to_handover(&self, gw_id: String, mqtt_payload_str: String) {
            let handover_topic = format!("{}/{}", self.mqtt_handover_base_topic, gw_id);
            let mqtt_handover_topic =
                mqtt::Topic::new(&self.mqtt_client, handover_topic, self.mqtt_qos);
            let tok: mqtt::DeliveryToken = mqtt_handover_topic.publish(mqtt_payload_str);
            if let Err(e) = tok.wait() {
                println!("Error sending message: {:?}", e);
            }
        }

        pub async fn publish_to_process(&self, mqtt_payload_str: String) {
            let mqtt_process_topic = mqtt::Topic::new(
                &self.mqtt_client,
                self.mqtt_process_topic.clone(),
                self.mqtt_qos,
            );
            let tok: mqtt::DeliveryToken = mqtt_process_topic.publish(mqtt_payload_str);
            if let Err(e) = tok.await {
                println!("Error sending message: {:?}", e);
            }
        }

        pub async fn publish_to_control(&self, command: String, mqtt_payload_str: String) {
            println!("INFO: Sending command: {}", command);
            println!("INFO: Payload: {}", mqtt_payload_str);
            let topic_string = format!("{}/up/{}", self.mqtt_control_topic, command);
            println!("INFO: Topic: {}", topic_string);
            let mqtt_control_topic =
                mqtt::Topic::new(&self.mqtt_client, topic_string, self.mqtt_qos);
            let tok: mqtt::DeliveryToken = mqtt_control_topic.publish(mqtt_payload_str);
            if let Err(e) = tok.await {
                println!("Error sending message: {:?}", e);
            }
        }

        pub async fn run_handover_client(&mut self) {
            let subscribe_topic: String = format!("{}", self.mqtt_handover_base_topic.clone());
            // let subscribe_topic: String =
            //     format!("{}/{}", self.mqtt_handover_base_topic.clone(), self.gw_id);
            let mut strm = self.mqtt_client.get_stream(128);
            self.mqtt_client.subscribe(subscribe_topic, self.mqtt_qos);

            if let Err(err) = block_on(async {
                while let Some(msg_opt) = strm.next().await {
                    match msg_opt {
                        Some(msg) => {
                            let msg_str = msg.payload_str();
                            let topic = msg.topic();
                            let e2l_crypto = self.e2l_crypto.lock().expect("Could not lock!");
                            let ret = e2l_crypto
                                .handover_callback(topic.to_string(), msg_str.to_string());
                            std::mem::drop(e2l_crypto);
                            match ret {
                                Some(payload) => {
                                    let mut counters = FRAME_COUNTERS.lock().unwrap();
                                    counters.rx_ho_frames += 1;
                                    counters.proc_frames +=1;
                                    self.publish_to_process(payload).await;
                                },
                                None => (),
                            }
                        }
                        None => {
                            println!("Lost connection. Attempting reconnect.");
                            while let Err(err) = self.mqtt_client.reconnect().await {
                                println!("Error reconnecting: {}", err);
                                // For tokio use: tokio::time::delay_for()
                                std::thread::sleep(Duration::from_millis(1000));
                            }
                        }
                    }
                }
                // Explicit return type for the async block
                Ok::<(), mqtt::Error>(())
            }) {
                println!("Error: {:?}", err)
            }
        }

        pub async fn run_control_client(&mut self) {
            let subscribe_topic: String =
                format!("{}/{}/down/+", self.gw_id, self.mqtt_control_topic);
            let mut strm = self.mqtt_client.get_stream(128);
            let _token = self.mqtt_client.subscribe(subscribe_topic, self.mqtt_qos);

            if let Err(err) = block_on(async {
                while let Some(msg_opt) = strm.next().await {
                    match msg_opt {
                        Some(msg) => {
                            println!("INFO: WELCOME TO THE EMQX COMMANDS:))))");
                            let payload_str = msg.payload_str().to_string();
                            let topic = msg.topic();
                            // Split topic at /
                            let topic_parts: Vec<&str> = topic.split("/").collect();
                            // get last elem
                            let command = topic_parts[topic_parts.len() - 1];
                            match command {
                                "add_assigned_devices" => {
                                    println!("INFO: Command 'add_assigned_devices' received");
                                    let devices_result: Result<Vec<NewAssignedDevice>, Error> =
                                        serde_json::from_str(&payload_str);
                                    match devices_result {
                                        Ok(devices) => {
                                            let devices_len = devices.len();
                                            let e2l_crypto =
                                                self.e2l_crypto.lock().expect("Could not lock!");
                                            for device in devices {
                                                e2l_crypto.add_assigned_device(device);
                                            }
                                            std::mem::drop(e2l_crypto);
                                            println!(
                                                "INFO: {} assigned devices added",
                                                devices_len
                                            );
                                        }
                                        Err(_) => {
                                            println!("ERROR: Invalid JSON format for 'add_assigned_device' command");
                                        }
                                    }
                                }
                                "add_assigned_device" => {
                                    println!("INFO: Command 'add_assigned_device' received");
                                    let device_result: Result<NewAssignedDevice, Error> =
                                        serde_json::from_str(&payload_str);
                                    match device_result {
                                        Ok(device) => {
                                            let e2l_crypto =
                                                self.e2l_crypto.lock().expect("Could not lock!");
                                            e2l_crypto.add_assigned_device(device);
                                            std::mem::drop(e2l_crypto);
                                            println!("INFO: Assigned device added");
                                        }
                                        Err(_) => {
                                            println!("ERROR: Invalid JSON format for 'add_assigned_device' command");
                                        }
                                    }
                                }
                                "add_unassigned_devices" => {
                                    println!("INFO: Command 'add_unassigned_devices' received");
                                    let devices_result: Result<Vec<NewUnassociatedDevice>, Error> =
                                        serde_json::from_str(&payload_str);
                                    match devices_result {
                                        Ok(devices) => {
                                            let mut assigned_gws: HashSet<String> = HashSet::new();
                                            let devices_len = devices.len();
                                            for device in devices {
                                                assigned_gws.insert(device.assigned_gw.clone());
                                                let e2l_crypto = self
                                                    .e2l_crypto
                                                    .lock()
                                                    .expect("Could not lock!");
                                                e2l_crypto.add_unassigned_device(device);
                                                std::mem::drop(e2l_crypto);
                                            }
                                            println!(
                                                "INFO: {} unassigned devices added",
                                                devices_len
                                            );
                                            for assigned_gw in assigned_gws {
                                                self.create_gw_bridge(assigned_gw).await;
                                            }
                                            println!("INFO: Bridge created for assigned gateways");
                                        }
                                        Err(_) => {
                                            println!("ERROR: Invalid JSON format for 'add_unassigned_device' command");
                                        }
                                    }
                                }
                                "add_unassigned_device" => {
                                    println!("INFO: Command 'add_unassigned_device' received");
                                    let device_result: Result<NewUnassociatedDevice, Error> =
                                        serde_json::from_str(&payload_str);
                                    match device_result {
                                        Ok(device) => {
                                            let assigned_gw = device.assigned_gw.clone();
                                            let e2l_crypto =
                                                self.e2l_crypto.lock().expect("Could not lock!");
                                            e2l_crypto.add_unassigned_device(device);
                                            std::mem::drop(e2l_crypto);
                                            self.create_gw_bridge(assigned_gw).await;
                                        }
                                        Err(_) => {
                                            println!("ERROR: Invalid JSON format for 'add_unassigned_device' command");
                                        }
                                    }
                                }
                                "remove_assigned_device" => {
                                    println!("INFO: Command 'remove_assigned_device' received");
                                    let device_result: Result<RemoveDevice, Error> =
                                        serde_json::from_str(&payload_str);
                                    match device_result {
                                        Ok(device) => {
                                            let e2l_crypto =
                                                self.e2l_crypto.lock().expect("Could not lock!");
                                            e2l_crypto.remove_assigned_device(device.dev_addr);
                                            std::mem::drop(e2l_crypto);
                                        }
                                        Err(_) => {
                                            println!("ERROR: Invalid JSON format for 'remove_assigned_device' command");
                                        }
                                    }
                                }
                                "remove_unassigned_device" => {
                                    println!("INFO: Command 'remove_unassigned_device' received");
                                    let device_result: Result<RemoveDevice, Error> =
                                        serde_json::from_str(&payload_str);
                                    match device_result {
                                        Ok(device) => {
                                            let e2l_crypto =
                                                self.e2l_crypto.lock().expect("Could not lock!");
                                            e2l_crypto.remove_unassigned_device(device.dev_addr);
                                            std::mem::drop(e2l_crypto);
                                        }
                                        Err(_) => {
                                            println!("ERROR: Invalid JSON format for 'remove_assigned_device' command");
                                        }
                                    }
                                }
                                "set_active" => {
                                    println!("INFO: Command 'set_active' received");
                                    let active_result: Result<GwActiveStatus, Error> =
                                        serde_json::from_str(&payload_str);

                                    match active_result {
                                        Ok(gw_active_status) => {
                                            let is_active = gw_active_status.active;
                                            let e2l_crypto =
                                                self.e2l_crypto.lock().expect("Could not lock!");
                                            e2l_crypto.set_active(is_active);
                                            std::mem::drop(e2l_crypto);
                                        }
                                        Err(_) => {
                                            println!("ERROR: Invalid JSON format for 'set_active' command");
                                        }
                                    }
                                }
                                "aggregation_completed" => {
                                    println!("INFO: Command 'aggregation_completed' received");
                                }
                                _ => {
                                    println!("INFO: Unknown command received");
                                }
                            }
                        }
                        None => {
                            println!("Lost connection. Attempting reconnect.");
                            while let Err(err) = self.mqtt_client.reconnect().await {
                                println!("Error reconnecting: {}", err);
                                // For tokio use: tokio::time::delay_for()
                                std::thread::sleep(Duration::from_millis(1000));
                            }
                        }
                    }
                }
                // Explicit return type for the async block
                Ok::<(), mqtt::Error>(())
            }) {
                println!("Error: {:?}", err)
            }
        }

        pub async fn create_as_bridge(&mut self) -> bool {
            println!("Creating bridge for gateway: {}", self.gw_id);
            let url = format!("{}bridges", self.api_endpoint);
            let topic_wildcard = "${topic}".to_string();
            let server = format!(
                "{}:{}",
                env::var("MQTT_SINK_HOST").unwrap(),
                env::var("MQTT_SINK_PORT").unwrap()
            );

            // Create data bridge
            let name = format!("{}-as-data-bridge", self.gw_id);
            let local_topic = env::var("MQTT_TOPIC_OUTPUT").unwrap();
            let remote_topic = format!("{}/{}", self.gw_id, topic_wildcard.clone());
            let config = MqttBridgeEgressConfig::new(
                name,
                server.clone(),
                self.api_username.clone(),
                self.api_password.clone(),
                local_topic,
                remote_topic,
            );
            let response_result = Client::new()
                .post(url.clone())
                .basic_auth(self.api_username.clone(), Some(self.api_password.clone()))
                .json(&config)
                .send()
                .await;
            if let Err(e) = response_result {
                println!("Error creating bridge: {:?}", e);
                return false;
            }
            let response = response_result.unwrap();
            if response.status().is_success() {
                println!("Bridge created successfully");
            } else {
                let text = response.text().await.unwrap();
                if text.contains("ALREADY_EXISTS") {
                    println!("Bridge already exists");
                } else {
                    println!("Error creating bridge: {:?}", text);
                    return false;
                }
            }

            // Create control bridge
            let name_egress = format!("{}-as-control-bridge-egress", self.gw_id);
            // let name_ingress = format!("{}-as-control-bridge-ingress", self.gw_id);
            let local_topic = format!("{}/up/+", self.mqtt_control_topic.clone());
            println!("INFO: Local topic: {}", local_topic);
            let remote_topic = format!("{}/{}", self.gw_id, topic_wildcard.clone());
            println!("INFO: Remote topic: {}", remote_topic);
            let egress_config = MqttBridgeEgressConfig::new(
                name_egress.clone(),
                server.clone(),
                self.api_username.clone(),
                self.api_password.clone(),
                local_topic.clone(),
                remote_topic.clone(),
            );
            let egress_response_result = Client::new()
                .post(url.clone())
                .basic_auth(self.api_username.clone(), Some(self.api_password.clone()))
                .json(&egress_config)
                .send()
                .await;
            if let Err(e) = egress_response_result {
                println!("Error creating egress bridge: {:?}", e);
                return false;
            }
            let egress_response = egress_response_result.unwrap();
            if egress_response.status().is_success() {
                println!("Egress bridge created successfully");
                // print response
                println!("{:?}", egress_response);
            } else {
                let text = egress_response.text().await.unwrap();
                if text.contains("ALREADY_EXISTS") {
                    println!("Bridge already exists");
                } else {
                    println!("Error creating bridge: {:?}", text);
                    return false;
                }
            }
            true
        }

        async fn create_gw_bridge(&mut self, gw_id: String) -> bool {
            println!("Creating bridge for gateway hadover: {}", gw_id.clone());
            let url = format!("{}bridges", self.api_endpoint);
            // let topic_wildcard = "${topic}".to_string();
            let mut fqdn = gw_id.clone();
            let gw_service_name_env = env::var("GW_SERVICE_NAME");
            match gw_service_name_env {
                Ok(gw_service_name) => {
                    fqdn = format!("{}.{}", gw_id.clone(), gw_service_name);
                }
                Err(_) => {}
            }
            let server = format!("{}:{}", fqdn, env::var("BROKER_PORT").unwrap());

            // Create control bridge
            let name_egress = format!("{}-gw-handover-bridge-egress", gw_id.clone());
            let local_topic = format!("{}/+", self.mqtt_handover_base_topic.clone());
            println!("INFO: Local topic: {}", local_topic);
            let remote_topic = format!("{}", self.mqtt_handover_base_topic.clone());
            println!("INFO: Remote topic: {}", remote_topic);
            let egress_config = MqttBridgeEgressConfig::new(
                name_egress.clone(),
                server.clone(),
                self.api_username.clone(),
                self.api_password.clone(),
                local_topic.clone(),
                remote_topic.clone(),
            );
            let egress_response_result = Client::new()
                .post(url.clone())
                .basic_auth(self.api_username.clone(), Some(self.api_password.clone()))
                .json(&egress_config)
                .send()
                .await;
            if let Err(e) = egress_response_result {
                println!("Error creating egress bridge: {:?}", e);
                return false;
            }
            let egress_response = egress_response_result.unwrap();
            if egress_response.status().is_success() {
                println!("Egress bridge created successfully");
                // print response
                // println!("{:?}", egress_response);
            } else {
                let text = egress_response.text().await.unwrap();
                if text.contains("ALREADY_EXISTS") {
                    println!("Bridge already exists");
                } else {
                    println!("Error creating bridge: {:?}", text);
                    return false;
                }
            }
            true
        }
    }
}
