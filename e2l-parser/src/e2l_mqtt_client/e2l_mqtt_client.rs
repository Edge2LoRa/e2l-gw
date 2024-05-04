pub(crate) mod e2l_mqtt_client {
    use futures::{executor::block_on, stream::StreamExt};
    use std::sync::{Arc, Mutex};

    use crate::{
        e2l_crypto::e2l_crypto::e2l_crypto::E2LCrypto,
        mqtt_client::mqtt_structs::mqtt_structs::MqttVariables,
    };
    use paho_mqtt as mqtt;
    use std::time::Duration;

    pub struct E2LMqttClient {
        mqtt_client: mqtt::AsyncClient,
        mqtt_process_topic: String,
        mqtt_handover_base_topic: String,
        mqtt_conn_opts_builder: mqtt::ConnectOptionsBuilder,
        mqtt_qos: i32,
        e2l_crypto: Arc<Mutex<E2LCrypto>>,
    }

    impl E2LMqttClient {
        pub fn new(mqtt_variables: MqttVariables, e2l_crypto: Arc<Mutex<E2LCrypto>>) -> Self {
            let mqtt_client: mqtt::AsyncClient = mqtt::AsyncClient::new(format!(
                "{}:{}",
                mqtt_variables.broker_url.clone(),
                mqtt_variables.broker_port.clone()
            ))
            .unwrap_or_else(|err| {
                println!("Error creating the client: {:?}", err);
                std::process::exit(1);
            });

            // Connection options
            let mut mqtt_conn_opts_builder: mqtt::ConnectOptionsBuilder =
                mqtt::ConnectOptionsBuilder::new();
            mqtt_conn_opts_builder.user_name(mqtt_variables.broker_auth_name.clone());
            mqtt_conn_opts_builder.password(mqtt_variables.broker_auth_password.clone());

            // Subscribe to HANDOVER TOPIC
            let handover_base_topic = mqtt_variables.broker_handover_topic.clone();
            E2LMqttClient {
                mqtt_client: mqtt_client,
                mqtt_process_topic: mqtt_variables.broker_process_topic,
                mqtt_handover_base_topic: handover_base_topic,
                mqtt_qos: mqtt_variables.broker_qos,
                mqtt_conn_opts_builder: mqtt_conn_opts_builder,
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

        pub fn publish_to_process(&self, mqtt_payload_str: String) {
            let mqtt_process_topic = mqtt::Topic::new(
                &self.mqtt_client,
                self.mqtt_process_topic.clone(),
                self.mqtt_qos,
            );
            let tok: mqtt::DeliveryToken = mqtt_process_topic.publish(mqtt_payload_str);
            if let Err(e) = tok.wait() {
                println!("Error sending message: {:?}", e);
            }
        }

        pub async fn run(&mut self) {
            let mut strm = self.mqtt_client.get_stream(25);
            let _ = self
                .mqtt_client
                .connect(self.mqtt_conn_opts_builder.finalize())
                .await;
            self.mqtt_client
                .subscribe(self.mqtt_handover_base_topic.clone(), self.mqtt_qos);

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
                                Some(payload) => self.publish_to_process(payload),
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
    }
}
