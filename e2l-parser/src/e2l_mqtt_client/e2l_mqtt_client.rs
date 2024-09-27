pub(crate) mod e2l_mqtt_client {
    extern crate p256;

    use futures::{executor::block_on, stream::StreamExt};
    use serde_derive::Deserialize;
    use serde_derive::Serialize;
    use serde_json::Error;
    use std::sync::{Arc, Mutex};

    use crate::e2l_crypto::e2l_crypto::e2l_crypto::E2LCrypto;
    use paho_mqtt as mqtt;
    use std::time::Duration;

    #[derive(Debug, Serialize, Deserialize)]
    pub struct MqttVariables {
        pub broker_url: String,
        pub broker_port: String,
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

    pub struct E2LMqttClient {
        gw_id: String,
        mqtt_client: mqtt::AsyncClient,
        mqtt_process_topic: String,
        mqtt_handover_base_topic: String,
        mqtt_control_topic: String,
        mqtt_qos: i32,
        e2l_crypto: Arc<Mutex<E2LCrypto>>,
    }

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
        pub qos: u32,
        pub retain: String,
        pub payload: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct MqttBridgeLocalOpts {
        pub topic: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct MqttBridgeEgressOpts {
        pool_size: u32,
        remote: MqttBridgeRemoteOpts,
        local: MqttBridgeLocalOpts,
    }
    #[derive(Debug, Serialize, Deserialize)]
    pub struct MqttBridgeIngressOpts {
        pool_size: u32,
        local: MqttBridgeRemoteOpts,
        remote: MqttBridgeLocalOpts,
    }
        
    #[derive(Debug, Serialize, Deserialize)]
    pub struct MqttBridgeEgressConfig {
        pub name: String,
        pub type_: String,
        pub enable: bool,
        pub resource_opts: MqttBridgeResourceOpts,
        pub server: String,
        pub proto_ver: String,
        pub username: String,
        pub password: String,
        pub ssl: MqqtBridgeSslConfig,
        pub egress: MqttBridgeEgressOpts,
    }
    // TODO: FINISH IMPLEMENTING THE DEFAULT FOR THE STRUCTS



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
            E2LMqttClient {
                gw_id: gw_id,
                mqtt_client: mqtt_client,
                mqtt_process_topic: mqtt_variables.broker_process_topic,
                mqtt_handover_base_topic: handover_base_topic,
                mqtt_control_topic: mqtt_variables.broker_control_topic,
                mqtt_qos: mqtt_variables.broker_qos,
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

        pub async fn _publish_to_control(&self, command: String, mqtt_payload_str: String) {
            let topic_string = format!("{}/{}", self.mqtt_control_topic, command);
            let mqtt_control_topic =
                mqtt::Topic::new(&self.mqtt_client, topic_string, self.mqtt_qos);
            let tok: mqtt::DeliveryToken = mqtt_control_topic.publish(mqtt_payload_str);
            if let Err(e) = tok.await {
                println!("Error sending message: {:?}", e);
            }
        }

        pub async fn run_handover_client(&mut self) {
            let subscribe_topic: String =
                format!("{}/{}", self.mqtt_handover_base_topic.clone(), self.gw_id);
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
                                Some(payload) => self.publish_to_process(payload).await,
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
            let subscribe_topic: String = format!("{}", self.mqtt_control_topic);
            let mut strm = self.mqtt_client.get_stream(128);
            let _token = self.mqtt_client.subscribe(subscribe_topic, self.mqtt_qos);

            if let Err(err) = block_on(async {
                while let Some(msg_opt) = strm.next().await {
                    match msg_opt {
                        Some(msg) => {
                            let payload_str = msg.payload_str().to_string();
                            let topic = msg.topic();
                            // Split topic at /
                            let topic_parts: Vec<&str> = topic.split("/").collect();
                            // get last elem
                            let command = topic_parts[topic_parts.len() - 1];
                            match command {
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
                                "add_unassigned_device" => {
                                    println!("INFO: Command 'add_unassigned_device' received");
                                    let device_result: Result<NewUnassociatedDevice, Error> =
                                        serde_json::from_str(&payload_str);
                                    match device_result {
                                        Ok(device) => {
                                            let e2l_crypto =
                                                self.e2l_crypto.lock().expect("Could not lock!");
                                            e2l_crypto.add_unassigned_device(device);
                                            std::mem::drop(e2l_crypto);
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
    }
}
