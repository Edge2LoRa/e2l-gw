pub mod traits;
pub(crate) mod e2l_module {
    use crate::e2l_mqtt_client::e2l_mqtt_client::{E2LMqttClient, GWPubInfo, FRAME_COUNTERS};
    use crate::e2l_mqtt_client::e2l_mqtt_client::{GwStats, MqttVariables, CombinedStats, FrameCounters};
    use crate::lorawan_structs::lora_structs::{Rxpk, RxpkContent};
    use crate::lorawan_structs::ForwardProtocols;
    use crate::{
        e2l_crypto::e2l_crypto::E2LCrypto,
        filters_json_structs::filter_json::EnvVariables,
        lorawan_structs::ForwardInfo,
    };
    use crate::e2l_end_device::e2l_end_device::{DeviceStats, DevicePks};
    use gethostname::gethostname;
    use lorawan_encoding::default_crypto::DefaultFactory;
    use lorawan_encoding::parser::{
        parse, AsPhyPayloadBytes, DataHeader, DataPayload, EncryptedDataPayload, PhyPayload,
    };
    use rand::Rng;
    use std::collections::{HashMap, HashSet};  
    use ordered_float::OrderedFloat;
    use std::time::{Duration};
    use sysinfo::{CpuExt, System, SystemExt};
    // use std::io::Read;
    use std::str;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};
    // BASE 64
    use base64::{engine::general_purpose, Engine as _};

    // RPC
    use lazy_static::lazy_static;
    use std::{net::UdpSocket, sync::mpsc::channel, thread};

    //Network
    use super::traits::NtwkStats;

    /********************
     * STATIC VARIABLES *
     ********************/
    const TIMEOUT: u64 = 3 * 60 * 100;
    static mut DEBUG: bool = false;

    // PORTS
    static DEFAULT_APP_PORT: u8 = 2;
    static DEFAULT_E2L_APP_PORT: u8 = 4;

    lazy_static! {
        static ref PACKETNAMES: HashMap<u8, &'static str> = {
            let mut m = HashMap::new();
            m.insert(0, "PUSH_DATA");
            m.insert(1, "PUSH_ACK");
            m.insert(2, "PULL_DATA");
            m.insert(3, "PULL_RESP");
            m.insert(4, "PULL_ACK");
            m.insert(5, "TX_ACK");
            m
        };
        static ref COUNT: usize = PACKETNAMES.len();
    }
    pub struct E2LModule {
        hostname: Arc<Mutex<String>>,
        fwinfo: Arc<Mutex<ForwardInfo>>,
        e2l_crypto: Arc<Mutex<E2LCrypto>>,
    }
    // STATIC FUNCTION
    impl E2LModule {
        fn debug(msg: String) {
            if unsafe { DEBUG } {
                println!("DEBUG: {}", msg);
            }
        }

        fn info(msg: String) {
            if true {
                //         println!("\nINFO: {}\n", msg);
                println!("INFO: {}", msg);
            }
        }
        fn charge_mqtt_variables() -> MqttVariables {
            MqttVariables {
                broker_url: dotenv::var("BROKER_URL").unwrap(),
                broker_port: dotenv::var("BROKER_PORT").unwrap(),
                broker_api_port: dotenv::var("BROKER_API_PORT").unwrap(),
                broker_auth_name: dotenv::var("BROKER_AUTH_USERNAME").unwrap(),
                broker_auth_password: dotenv::var("BROKER_AUTH_PASSWORD").unwrap(),
                broker_process_topic: dotenv::var("BROKER_PROCESS_TOPIC").unwrap(),
                broker_control_topic: dotenv::var("BROKER_CONTROL_TOPIC").unwrap(),
                broker_handover_topic: dotenv::var("BROKER_HANDOVER_BASE_TOPIC").unwrap(),
                broker_qos: dotenv::var("BROKER_QOS").unwrap().parse::<i32>().unwrap(),
            }
        }
        fn charge_environment_variables() -> EnvVariables {
            EnvVariables {
                local_port: dotenv::var("AGENT_PORT").unwrap(),
                remote_port: dotenv::var("NB_PORT").unwrap(),
                remote_addr: dotenv::var("NB_HOST").unwrap(),
                bind_addr: dotenv::var("AGENT_BIND_ADDR").unwrap(),
                filters: dotenv::var("FILE_AND_PATH").unwrap(),
                debug: if dotenv::var("DEBUG").unwrap().is_empty() {
                    false
                } else {
                    dotenv::var("DEBUG").unwrap().parse().unwrap()
                },
            }
        }
        fn get_data_from_json(from_upstream: &[u8]) -> Rxpk {
            // Some JSON input data as a &str. Maybe this comes from the user.
            let data_string = str::from_utf8(from_upstream).unwrap();
            Self::debug(format!("{}", data_string));
            let data: Rxpk = match serde_json::from_str(data_string) {
                Ok(data) => data,
                Err(e) => {
                    Self::debug(format!("Rxpk not present in JSON: {:?}", e));
                    Rxpk { rxpk: vec![] }
                }
            };
            data
        }
        fn extract_dev_addr_array(v: Vec<u8>) -> [u8; 4] {
            let default_array: [u8; 4] = [0, 0, 0, 0];
            v.try_into().unwrap_or(default_array)
        }
        
    }

    // PRIVATE FUNCTIONS
    impl E2LModule {
        async fn start_gw_stats_thread(&self) -> Arc<Mutex<Option<GwStats>>> {
            // Passing the GW status as an object to the load_balancer_interface()
            // Create a thread-safe, shared container for `GwStats`.
            let shared_stats: Arc<Mutex<Option<GwStats>>> = Arc::new(Mutex::new(None));
            // `shared_clone` is a cloned handle to the same shared state, allowing
            // another thread to update or read the stats concurrently.
            let shared_clone = Arc::clone(&shared_stats);

            Self::info(format!("Starting System counter stats thread!"));
            let hostname_mut = self.hostname.lock().expect("Could not lock!");
            let hostname = hostname_mut.clone();
            std::mem::drop(hostname_mut);
            let e2l_crypto_clone_publisher = Arc::clone(&self.e2l_crypto);
            let hostname_mut = self.hostname.lock().expect("Could not lock!");
            let hostname = hostname.clone();
            std::mem::drop(hostname_mut);
            let mqtt_variables: MqttVariables = Self::charge_mqtt_variables();

           

            thread::spawn(move || {
                let mut s: System = System::new_all();
            
        
                Self::info(format!("System counter stats thread started!"));

                let _mqtt_client = E2LMqttClient::new(
                    hostname.clone(),
                    "sys_stats_publisher".to_string(),
                    mqtt_variables,
                    e2l_crypto_clone_publisher,
                );
                

                loop {
                    println!("GATEWAY STATS: STARTING COLLECTING METRICS!");
                    s.refresh_memory();
                    // Get the total network usage
                    s.refresh_networks(); 
                    let mut sys = System::new_all(); // must be mutable!
                    let up_kb = sys.get_ntwk_up(); 
                    let down_kb = sys.get_ntwk_down();
                    // Get memory & Swap usage
                    let used_swap=s.swap_used();
                    // Get Cpu usage
                    s.refresh_cpu(); // Refreshing CPU information.
                    let used_cpu = s.global_cpu_info().cpu_usage();
                    Self::debug(format!("{}%", used_cpu));
                    let counters = FRAME_COUNTERS.lock().unwrap();
                    let gw_stats_obj = GwStats {
                        gw_id: hostname.clone(),
                        frame: FrameCounters {
                            rx_frames: counters.rx_frames,
                            fw_frames: counters.fw_frames,
                            tx_ho_frames: counters.tx_ho_frames,
                            rx_ho_frames: counters.rx_ho_frames,
                            proc_frames: counters.proc_frames,
                        },
                        mem_usage: s.used_memory(),
                        mem_usage_percentage: s.used_memory()*100,
                        mem_available: s.available_memory(),
                        ntwk_down: down_kb,
                        ntwk_up: up_kb,
                        cpu_usage: used_cpu,
                        cpu_usage_percentage: used_cpu*100.0,
                        swp_usage_percentage: used_swap,
                    };
                    //lock the thread and make clone out of gw_stats_obj
                    let mut lock = shared_clone.lock().unwrap();
                    *lock = Some(gw_stats_obj.clone());



                    let gw_stats_str = serde_json::to_string(&gw_stats_obj).unwrap();
                    // let _ = mqtt_client.publish_to_process(gw_stats_str);
                   println!("GATEWAY STATS: {}", gw_stats_str);
                   println!("GATEWAY STATS: SLEEPING FOR FIVE SECONDS");
                   thread::sleep(Duration::from_millis(5000));
                   
                }
            });
            shared_stats

        }
        async fn collect_packets_for_devices(&self,all_packets: Vec<RxpkContent>) -> HashMap<String, DevicePks> {
            let mut device_map: HashMap<String, DevicePks> = HashMap::new();

            for rxpk in all_packets {
                        let dev_eui = hex::encode(&rxpk.data[0..8]);
                        let dev_addr = hex::encode(&rxpk.data[8..12]);
                        let device_key = dev_addr.clone();
                        
                        //If the device already exists in the map, add this new packet to its list. 
                        //If it doesn't exist, insert a new device with the packet as its first entry.
                        device_map
                        .entry(device_key.clone())
                        .and_modify(|device| {
                            device.rxpk.push(rxpk.clone());
                            device.modu_set.insert(rxpk.modu.clone());
                            device.freq_set.insert(OrderedFloat(rxpk.freq));

                            if let Some(chan) = rxpk.chan {
                                device.chan_set.insert(chan);
                            }


                            if let Some((sf, bw)) = DevicePks::parse_datr(&rxpk.datr) {
                                device.sf_set.insert(sf);
                                device.bw_set.insert(bw);
                            } else {
                                println!("Invalid datr format: {}", rxpk.datr);
                            }
                        })
                        .or_insert(DevicePks {
                            dev_eui,
                            dev_addr,
                            rxpk: vec![rxpk.clone()],
                            modu_set: {
                                let mut m = HashSet::new();
                                m.insert(rxpk.modu.clone());
                                m
                            },
                            freq_set: {
                                let mut f = HashSet::new();
                                f.insert(OrderedFloat(rxpk.freq));
                                f
                            },
                            chan_set: {
                                let mut ch = HashSet::new();
                                if let Some(chan) = rxpk.chan {
                                    ch.insert(chan); 
                                }
                                ch
                            },
                            sf_set: {
                                let mut sf = HashSet::new();
                                if let Some((parsed_sf, _)) = DevicePks::parse_datr(&rxpk.datr) {
                                    sf.insert(parsed_sf);
                                }
                                sf
                            },
                            bw_set: {
                                let mut bw = HashSet::new();
                                if let Some((_, parsed_bw)) = DevicePks::parse_datr(&rxpk.datr) {
                                    bw.insert(parsed_bw);
                                }
                                bw
                            },
                        });
            }

            device_map
        }
        async fn calculate_device_stats(&self,device_map: HashMap<String, DevicePks>) -> HashMap<String,DeviceStats> {
            let mut stats_list: HashMap<String, DeviceStats> = HashMap::new();

            for (_dev_addr, device) in device_map.into_iter() {
                // Use the new methods
                let avg_rssi = match device.avg_rssi() {
                    Some(val) => val,
                    None => continue,
                };

                let avg_snr = device.avg_snr().unwrap_or(0.0);
                let avg_payload_size = device.avg_payload_size().unwrap_or(0.0);
                
                let counters = FRAME_COUNTERS.lock().unwrap();
                let stats = DeviceStats {
                    dev_eui: device.dev_eui,
                    frames:FrameCounters { rx_frames: counters.rx_frames, fw_frames: counters.fw_frames, tx_ho_frames: counters.tx_ho_frames, rx_ho_frames:counters.rx_ho_frames, proc_frames: counters.proc_frames },
                    fcnt: 1, 
                    dev_addr: device.dev_addr, 
                    avg_rssi,
                    avg_snr,
                    avg_payload_size,
                    modu: device.modu_set.clone(),
                    freq: device.freq_set.clone(),
                    chan: device.chan_set.clone(),
                    sf:device.sf_set.clone(),
                    bw:device.bw_set.clone()
                };

                
                stats_list.insert(_dev_addr, stats);
                let _stats_list_str = serde_json::to_string(&stats_list).unwrap();
            }
            stats_list
        }
        async fn handle_data_payload(
            &self,
            phy: EncryptedDataPayload<Vec<u8>, DefaultFactory>,
            packet: &RxpkContent,
            gwmac: String,
            mqtt_client: &E2LMqttClient,
        ) -> Option<bool> {
            let mut will_send: bool = false;
            let fhdr = phy.fhdr();
            let fcnt = fhdr.fcnt();
            let f_port = phy.f_port().unwrap();
            let dev_addr_vec = fhdr.dev_addr().as_ref().to_vec();
            let aux: Vec<u8> = dev_addr_vec.clone().into_iter().rev().collect();
            let strs: Vec<String> = aux.iter().map(|b| format!("{:02X}", b)).collect();
            let dev_addr_string = strs.join("");

            // let dev_addr_string = format!("{:x}", dev_addr_vec.clone());
            let _dev_addr = u32::from_be_bytes(Self::extract_dev_addr_array(
                dev_addr_vec.into_iter().rev().collect(),
            ));

            let is_active: bool;
            let e2l_crypto = self.e2l_crypto.lock().expect("Could not lock!");
            is_active = e2l_crypto.is_active();
            std::mem::drop(e2l_crypto);
            if is_active {
                // get epoch time
                let start = SystemTime::now();
                let _timetag = start
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards");

                // Check if enabled E2ED
                let e2l_crypto = self.e2l_crypto.lock().expect("Could not lock!");
                let e2ed_enabled: bool = (f_port == DEFAULT_E2L_APP_PORT)
                    && e2l_crypto.check_e2ed_enabled(dev_addr_string.clone());
                std::mem::drop(e2l_crypto);
                if e2ed_enabled {
                    let e2l_crypto = self.e2l_crypto.lock().expect("Could not lock!");
                    let mqtt_payload_option = e2l_crypto.get_json_mqtt_payload(
                        dev_addr_string.clone(),
                        fcnt,
                        phy,
                        packet,
                        gwmac,
                        None,
                    );
                    std::mem::drop(e2l_crypto);
                    ///////////////////////////////////////////////////////////////////////
                    // Processing for a device that is already enabled in E2L mode
                    // Increase frame for processing
                    ///////////////////////////////////////////////////////////////////////
                    match mqtt_payload_option {
                        Some(mqtt_payload) => {
                            let mut counters = FRAME_COUNTERS.lock().unwrap();
                            counters.proc_frames += 1;
                            let mqtt_payload_str = serde_json::to_string(&mqtt_payload)
                                .unwrap_or_else(|_| "Error".to_string());
                            mqtt_client
                                .publish_to_process(mqtt_payload_str.clone())
                                .await;
                        }
                        _ => {
                            println!("Error processing E2ED");
                            return None;
                        }
                    }
                    will_send = false;
                } else {
                    //////////////////////////////////////////////////////////////////////
                    // If the end-device is not enabled then we have to check the port
                    /////////////////////////////////////////////////////////////////////
                    match f_port {
                        port if port == DEFAULT_E2L_APP_PORT => {
                            let e2l_crypto = self.e2l_crypto.lock().expect("Could not lock!");
                            let mqtt_payload_option = e2l_crypto
                                .get_unassociated_json_mqtt_payload(
                                    dev_addr_string.clone(),
                                    fcnt,
                                    packet,
                                    gwmac,
                                );
                            println!("THAT IS THE PLACE THAT WE TOOK MQTT PAYLOAD NEW!{:?}",mqtt_payload_option);
                            std::mem::drop(e2l_crypto);
                            match mqtt_payload_option {
                                Some(mqtt_payload) => {
                                    let mut counters = FRAME_COUNTERS.lock().unwrap();
                                    let gw_id = mqtt_payload.gw_id.clone();
                                    let mqtt_payload_str = serde_json::to_string(&mqtt_payload)
                                        .unwrap_or_else(|_| "Error".to_string());
                                    mqtt_client.publish_to_handover(gw_id, mqtt_payload_str);
                                    counters.proc_frames += 1;
                                    will_send = false;
                                }
                                None => {}
                            }
                        }
                        port if port == DEFAULT_APP_PORT => {
                            println!("THAT IS THE NUMBER OF PORT:{}",port);
                            let fwinfo = self.fwinfo.lock().expect("Could not lock!");
                            let mut counters = FRAME_COUNTERS.lock().unwrap();
                            match fwinfo.forward_protocol {
                                ForwardProtocols::UDP => {
                                    Self::debug(format!(
                                        "Forwarding to NS: {:x?}",
                                        fwinfo.forward_host.clone()
                                    ));
                                    counters.fw_frames += 1;
                                } // _ => panic!("Forwarding protocol not implemented!"),
                            }

                            std::mem::drop(fwinfo);
                        }
                        _ => {}
                    }
                }
            } else {
                Self::debug(format!("Not forwarding packet, GW NOT Active",));
                return None;
            }
            return Some(will_send);
        } 
    }

    // PUBLIC FUNCTIONS
    impl E2LModule {
        pub async fn new() -> E2LModule {
            dotenv::dotenv().ok();
            let env_variables: EnvVariables = Self::charge_environment_variables();
            /*****************
             * ENV VARIABLES *
             *****************/
            let remote_port: u16 = env_variables.remote_port.parse().unwrap();
            let remote_host: String = env_variables.remote_addr.parse().unwrap();

            let fwinfo: ForwardInfo = ForwardInfo {
                forward_host: remote_host,
                port: remote_port,
                ..Default::default()
            };

            // GET HOSTNAME
            let hostname: String;
            let hostname_env = dotenv::var("HOSTNAME");
            match hostname_env {
                Ok(hostname_env) => {
                    hostname = hostname_env;
                }
                Err(_) => {
                    hostname = gethostname().into_string().unwrap();
                }
            }

            /*
             * E2LCrypto
             */
            let e2l_crypto = E2LCrypto::new(hostname.clone());
            let e2l_crypto_arc = Arc::new(Mutex::new(e2l_crypto));
            E2LModule {
                hostname: Arc::new(Mutex::new(hostname.clone())),
                fwinfo: Arc::new(Mutex::new(fwinfo)),
                e2l_crypto: e2l_crypto_arc,
            }
        }

        pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
            /**************************
             * MQTT BROKER CONNECTION *
             **************************/
            let hostname = self.hostname.lock().expect("Could not lock!");
            let hostname_publisher = hostname.clone();
            let hostname_control_client = hostname.clone();
            let hostname_handover_client = hostname.clone();
            std::mem::drop(hostname);
            let mqtt_variables: MqttVariables = Self::charge_mqtt_variables();
            let mqtt_port = mqtt_variables.broker_port.clone();
            let e2l_crypto_clone_publisher = Arc::clone(&self.e2l_crypto);
            let e2l_crypto_clone_control_client = Arc::clone(&self.e2l_crypto);
            let e2l_crypto_clone_handover_client = Arc::clone(&self.e2l_crypto);
            let mut mqtt_client = E2LMqttClient::new(
                hostname_publisher,
                "publisher".to_string(),
                mqtt_variables,
                e2l_crypto_clone_publisher,
            );
            /********************
             * CREATE AS BRIDGE *
             ********************/
            let response = mqtt_client.create_as_bridge().await;
            if !response {
                return Err("Unable to create AS bridge".into());
            }

            thread::spawn(|| {
                let mqtt_variables: MqttVariables = Self::charge_mqtt_variables();
                let mut control_mqtt_client = E2LMqttClient::new(
                    hostname_control_client,
                    "control_client".to_string(),
                    mqtt_variables,
                    e2l_crypto_clone_control_client,
                );
                let rt =
                    tokio::runtime::Runtime::new().expect("Failed to obtain a new RunTime object");
                rt.block_on(control_mqtt_client.run_control_client());
            });
            thread::spawn(|| {
                let mqtt_variables: MqttVariables = Self::charge_mqtt_variables();
                let mut handover_mqtt_client = E2LMqttClient::new(
                    hostname_handover_client,
                    "handover_client".to_string(),
                    mqtt_variables,
                    e2l_crypto_clone_handover_client,
                );
                let rt =
                    tokio::runtime::Runtime::new().expect("Failed to obtain a new RunTime object");
                rt.block_on(handover_mqtt_client.run_handover_client());
            });

            /***********************
             * SEND PUB INFO TO AS *
             ***********************/

            // Compute private ECC key
            let e2l_crypto = self.e2l_crypto.lock().expect("Could not lock!");
            let compressed_public_key = e2l_crypto.compressed_public_key.clone().unwrap();
            std::mem::drop(e2l_crypto);

            // SEND GW PUB INFO TO AS
            let gw_pub_info = GWPubInfo {
                mqtt_port: mqtt_port.clone(),
                pub_key: compressed_public_key.into_vec(),
            };
            let gw_pub_info_str =
                serde_json::to_string(&gw_pub_info).expect("Failed to serialize GWPubInfo");
            mqtt_client
                .publish_to_control("pub_info".to_string(), gw_pub_info_str)
                .await;

            /**********************
             * TTS UDP CONNECTION *
             **********************/
            let env_variables = Self::charge_environment_variables();

            let bind_addr: String = if env_variables.bind_addr.is_empty() {
                "127.0.0.1".to_owned()
            } else {
                env_variables.bind_addr.parse().unwrap()
            };
            let local_port: i32 = env_variables.local_port.parse().unwrap();
            let local_addr = format!("{}:{}", bind_addr, local_port);
            let local =
                UdpSocket::bind(&local_addr).expect(&format!("Unable to bind to {}", &local_addr));

            let fwinfo = self.fwinfo.lock().expect("Could not lock");
            Self::info(format!("Listening on {}", local.local_addr().unwrap()));
            Self::info(format!(
                "Forwarding to {}:{}",
                fwinfo.forward_host, fwinfo.port
            ));

            let remote_addr = format!("{}:{}", fwinfo.forward_host, fwinfo.port);
            std::mem::drop(fwinfo);

            let responder = local.try_clone().expect(&format!(
                "Failed to clone primary listening address socket {}",
                local.local_addr().unwrap()
            ));

            let (main_sender, main_receiver) = channel::<(_, Vec<u8>)>();
            thread::spawn(move || {
                Self::debug(format!(
                    "Started new thread to deal out responses to clients"
                ));
                loop {
                    let (dest, buf) = main_receiver.recv().unwrap();
                    //tx_frame increase
                    let to_send = buf.as_slice();
                    responder.send_to(to_send, dest).expect(&format!(
                        "Failed to forward response from upstream server to client {}",
                        dest
                    ));
                }
            });

            /******************
             * GW STATS LOOP *
             ******************/
            let shared_stats = self.start_gw_stats_thread().await;
            
            /*************
             * MAIN LOOP *
             *************/
            let mut client_map = HashMap::new();
            let mut buf = [0; 64 * 1024];
            // let mut client_map = Arc::new(Mutex::new(HashMap::new()));
            // let mut buf = [0u8; 1024];
            // println!("Buf{:?}", buf);
            

            let e2l_crypto = self.e2l_crypto.lock().expect("Could not lock");
            e2l_crypto.set_active(true);
            std::mem::drop(e2l_crypto);
            Self::info(format!("Starting Listening for incoming LoRaWAN packets!"));
            loop {
                let (num_bytes, src_addr) = local.recv_from(&mut buf).expect("Didn't receive data");
                //we create a new thread for each unique client
                let mut remove_existing = false;
                loop {
                    Self::debug(format!("Received packet from client {}", src_addr)); 
                    //rx_frame increase
                    let mut ignore_failure = true;
                    let client_id = format!("{}", src_addr);

                    if remove_existing {
                        Self::debug(format!("Removing existing forwarder from map."));
                        client_map.remove(&client_id);
                    }

                    let sender = client_map.entry(client_id.clone()).or_insert_with(|| {
                        //we are creating a new listener now, so a failure to send shoud be treated as an error
                        ignore_failure = false;

                        let local_send_queue = main_sender.clone();
                        let (sender, receiver) = channel::<Vec<u8>>();
                        let remote_addr_copy = remote_addr.clone();

                        thread::spawn(move || {
                            let mut rng = rand::thread_rng();

                            //regardless of which port we are listening to, we don't know which interface or IP
                            //address the remote server is reachable via, so we bind the outgoing
                            //connection to 0.0.0.0 in all cases.
                            let temp_outgoing_addr = format!("0.0.0.0:{}", rng.gen_range(50000, 59999));
                            Self::debug(format!("Establishing new forwarder for client {} on {}", src_addr, &temp_outgoing_addr));
                            let upstream_send = UdpSocket::bind(&temp_outgoing_addr)
                                .expect(&format!("Failed to bind to transient address {}", &temp_outgoing_addr));
                            let upstream_recv = upstream_send.try_clone()
                                .expect("Failed to clone client-specific connection to upstream!");

                            let mut timeouts: u64 = 0;
                            let timed_out = Arc::new(AtomicBool::new(false));

                            let local_timed_out = timed_out.clone();


                            thread::spawn(move || {
                                let mut from_upstream = [0; 64 * 1024];

                                upstream_recv.set_read_timeout(Some(Duration::from_millis(TIMEOUT + 100))).unwrap();
                                loop {
                                    match upstream_recv.recv_from(&mut from_upstream) {
                                        Ok((bytes_rcvd, _)) => {
                                            let to_send = from_upstream[..bytes_rcvd].to_vec();
                                            Self::debug(format!("Forwarding packet from client {} to upstream server", PACKETNAMES[&to_send[3]]));

                                            local_send_queue.send((src_addr, to_send))
                                                .expect("Failed to queue response from upstream server for forwarding!");
                                        }
                                        Err(_) => {
                                            if local_timed_out.load(Ordering::Relaxed) {
                                                Self::debug(format!("Terminating forwarder thread for client {} due to timeout", src_addr));
                                                break;
                                            }
                                        }
                                    };
                                }
                            });

                            loop {
                                match receiver.recv_timeout(Duration::from_millis(TIMEOUT)) {
                                    Ok(from_client) => {
                                        Self::debug(format!("Forwarding packet from client {} to upstream server", PACKETNAMES[&from_client[3]]));
                                        upstream_send.send_to(from_client.as_slice(), &remote_addr_copy)
                                            .expect(&format!("Failed to forward packet from client {} to upstream server!", src_addr));
                                        timeouts = 0; //reset timeout count
                                    }
                                    Err(_) => {
                                        timeouts += 1;
                                        if timeouts >= 10 {
                                            Self::debug(format!("Disconnecting forwarder for client {} due to timeout", src_addr));
                                            timed_out.store(true, Ordering::Relaxed);
                                            break;
                                        }
                                    }
                                };
                            }

                        });

                        sender
                    });

                    let to_send = buf[..num_bytes].to_vec();

                    let mut will_send = true;
                    let mut packets: Vec<RxpkContent> = Vec::new();
                    
                    match &to_send[3] {
                        // Scritto da Copilot: Match a single value to a single value to avoid a match on a slice of a single value and a single value slice. This is a bit of a hack, but it works. I'm sorry. I'm sorry. I'm sorry.
                        0 => {
                            // PUSH_DATA
                            let data_json: Rxpk = Self::get_data_from_json(&to_send[12..]);
                            Self::debug(format!(
                                "Evaluate if forwarding packet from client {:?} to upstream server",
                                data_json.rxpk
                            ));
                           
                            if data_json.rxpk.len() == 0 {
                                let fwinfo = self.fwinfo.lock().expect("Could not lock!");
                                match fwinfo.forward_protocol {
                                    ForwardProtocols::UDP => {
                                        Self::debug(format!(
                                            "Forwarding Other data to {:x?}",
                                            fwinfo.forward_host.clone()
                                        ));
                                    } // _ => panic!("Forwarding protocol not implemented!"),
                                }
                                std::mem::drop(fwinfo);
                            } else {
                                for packet in data_json.rxpk.iter() {
                                    let data: Vec<u8> =
                                        general_purpose::STANDARD.decode(&packet.data).unwrap();
                                

                                    let gwmac: String = hex::encode(&to_send[4..12]);
                                    Self::debug(format!("Extracted GwMac {:x?}", gwmac));
                                    
                                    let parsed_data = parse(data.clone());
                                    if data.len() < 26 {
                                         println!("Invalid data length: {}, skipping packet", data.len());
                                         continue;
                                    }
                                    let packet_copy = packet.clone();
                                    //Collect each 20 packets and do the calculation
                                    // if packets.len() < 21{
                                       packets.push(packet_copy);
                                    // }else {
                                    //    break;
                                    // }
                                    //pippo
                                    match parsed_data {
                                        Ok(PhyPayload::Data(DataPayload::Encrypted(phy))) => {
                                            println!("WE ARE RECEIVING PACKETS!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
                                            let mut counters = FRAME_COUNTERS.lock().unwrap();
                                            counters.rx_frames += 1;

                                            let will_send_option = self
                                                .handle_data_payload(
                                                    phy,
                                                    packet,
                                                    gwmac,
                                                    &mqtt_client,
                                                )
                                                .await;
                                            match will_send_option {
                                                Some(will_send_value) => {
                                                    will_send = will_send_value
                                                }
                                                None => {
                                                    will_send = false;
                                                    break;
                                                }
                                            }
                                        }
                                        Ok(PhyPayload::JoinRequest(phy)) => {
                                            let fwinfo =
                                            self.fwinfo.lock().expect("Could not lock!");
                                            match fwinfo.forward_protocol {
                                                ForwardProtocols::UDP => {
                                                        Self::debug(format!(
                                                        "Forwarding to {:x?}  JoinRequest with len {}",
                                                        fwinfo.forward_host.clone(),
                                                        phy.as_bytes().len()
                                                    ));
                                                } // _ => panic!("Forwarding protocol not implemented!"),
                                            }
                                            std::mem::drop(fwinfo);
                                        }
                                        Ok(_) => {}
                                        Err(_) => {
                                            Self::info(format!("Not forwarding packet from client {} to upstream server, Unknown Packet with size {}, data: {:x?}", PACKETNAMES[&to_send[3]], data.len(), data));
                                            will_send = false;
                                            break;
                                        }
                                    }
                                }
                            }
                            
                            
                        }
                        _ => (),
                        
                    }

                    let device_map = self.collect_packets_for_devices(packets).await; 
                    let device_stats=self.calculate_device_stats(device_map).await; 
                    
                    let gw_stats = {
                                let lock = shared_stats.lock().unwrap();
                                if let Some(stats) = &*lock {
                                    stats.clone()
                                } else {
                                    // handle the missing case (e.g., skip this cycle, log warning, etc.)
                                    continue; // or return early
                                }
                            };
                    
                    let stats = CombinedStats {
                        gw_stats,
                        devices_stats: device_stats,
                    };

                    println!("THE STATUS OF END-DEVICE AND GATEWAY {:?}", stats);
              
                    if will_send {
                        match sender.send(to_send.to_vec().clone()) {
                            Ok(_) => {
                                Self::debug(format!(
                                    "Forwarding {} ({}) to upstream server",
                                    //fw_frame increase
                                    PACKETNAMES[&to_send[3]], &to_send[3]
                                ));

                                break;
                            }
                            Err(_) => {
                                if !ignore_failure {
                                    panic!(
                                "Failed to send message to datagram forwarder for client {}",
                                client_id
                            );
                                }
                                //client previously timed out
                                Self::debug(format!(
                                    "New connection received from previously timed-out client {}",
                                    client_id
                                ));
                                remove_existing = true;
                                continue;
                            }
                        }
                    } else {
                        break;
                    }
                }
                
            }

            // Ok(())
        }
    }
}
