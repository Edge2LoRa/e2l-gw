use self::e2l_crypto::E2LCrypto;
static AVG_ID: u8 = 1;
static _SUM_ID: u8 = 2;
static _MIN_ID: u8 = 3;
static _MAX_ID: u8 = 4;
pub(crate) mod e2l_crypto {
    // Crypto
    extern crate p256;
    extern crate serde_json;

    use std::ops::Mul;
    use std::time::{SystemTime, UNIX_EPOCH};

    use lorawan_encoding::default_crypto::DefaultFactory;
    use lorawan_encoding::keys::AES128;
    use lorawan_encoding::parser::EncryptedDataPayload;
    use p256::elliptic_curve::point::AffineCoordinates;
    use p256::elliptic_curve::point::NonIdentity;
    use p256::elliptic_curve::rand_core::OsRng;
    use p256::elliptic_curve::AffinePoint;
    use p256::elliptic_curve::NonZeroScalar;
    use p256::elliptic_curve::PublicKey as P256PublicKey;
    use p256::elliptic_curve::SecretKey as P256SecretKey;
    use sha2::Digest;
    use sha2::Sha256;

    use crate::lorawan_structs::lorawan_structs::lora_structs::RxpkContent;
    use crate::mqtt_client::mqtt_structs::mqtt_structs::MqttJson;

    static AVG_ID: u8 = 1;
    static SUM_ID: u8 = 2;
    static MIN_ID: u8 = 3;
    static MAX_ID: u8 = 4;

    pub struct _AggregationResult {
        pub status_code: i64,
        pub dev_eui: String,
        pub dev_addr: String,
        pub aggregated_data: i64,
        pub fcnts: Vec<u64>,
    }

    pub struct _ProcessedFrameResult {
        pub timetag: u64,
        pub aggregation_option: Option<_AggregationResult>,
    }

    pub struct DevInfo {
        dev_eui: String,
        dev_addr: String,
        dev_public_key: P256PublicKey<p256::NistP256>,
        edge_s_enc_key: AES128,
        edge_s_int_key: AES128,
        values: Vec<i64>,
        _fcnts: Vec<u64>,
    }

    #[derive(Default)]
    pub struct E2LCrypto {
        pub private_key: Option<P256SecretKey<p256::NistP256>>,
        pub public_key: Option<P256PublicKey<p256::NistP256>>,
        pub compressed_public_key: Option<Box<[u8]>>,
        pub active_directory: Vec<DevInfo>,
        pub aggregation_function: u8,
        pub window_size: usize,
        pub is_active: bool,
    }

    impl E2LCrypto {
        pub fn set_active(&mut self, is_active: bool) {
            self.is_active = is_active;
        }

        pub fn _load_device_from_file(&mut self, path: String) {
            if path.is_empty() {
                return;
            }
            let json_file = std::fs::read_to_string(&path);
            match json_file {
                Ok(json_to_string) => {
                    let devices_list: serde_json::Value =
                        serde_json::from_str(&json_to_string).unwrap();
                    for device in devices_list.as_array().unwrap() {
                        let dev_eui: &str = device["ids"]["dev_eui"].as_str().unwrap();
                        let dev_addr: &str = device["session"]["dev_addr"].as_str().unwrap();
                        // let device_keys: serde_json::Value = device["session"]["keys"];

                        // Get session keys
                        let edge_s_enc_key_str = device["session"]["keys"]["app_s_key"]["key"]
                            .as_str()
                            .unwrap();
                        let mut edge_s_enc_key_bytes: [u8; 16] = [0; 16];
                        hex::decode_to_slice(edge_s_enc_key_str, &mut edge_s_enc_key_bytes)
                            .expect("Decoding failed");
                        let edge_s_enc_key = AES128::from(edge_s_enc_key_bytes.clone());
                        let edge_s_int_key_str = device["session"]["keys"]["f_nwk_s_int_key"]
                            ["key"]
                            .as_str()
                            .unwrap();
                        let mut edge_s_int_key_bytes: [u8; 16] = [0; 16];
                        hex::decode_to_slice(edge_s_int_key_str, &mut edge_s_int_key_bytes)
                            .expect("Decoding failed");
                        let edge_s_int_key = AES128::from(edge_s_int_key_bytes.clone());

                        // Create fake priv pub device key
                        let dev_fake_private_key = Some(P256SecretKey::random(&mut OsRng));
                        let dev_fake_public_key =
                            Some(dev_fake_private_key.clone().unwrap().public_key()).unwrap();

                        // Create DevInfo struct and add to active directory
                        let new_dev_info: DevInfo = DevInfo {
                            dev_eui: dev_eui.to_string(),
                            dev_addr: dev_addr.to_string(),
                            dev_public_key: dev_fake_public_key,
                            edge_s_enc_key: edge_s_enc_key,
                            edge_s_int_key: edge_s_int_key,
                            values: Vec::new(),
                            _fcnts: Vec::new(),
                        };
                        self.active_directory.push(new_dev_info);
                    }
                    println!("Devices loaded from file");
                }
                Err(err) => {
                    println!("{:?}", err);
                    return;
                }
            }
        }
        /*
           @brief: This function multiplies a scalar with a point on the curve
           @param scalar: the scalar to multiply as private key
           @param point: the point to multiply as public key
           @return: the result of the scalar multiplication
        */
        fn scalar_point_multiplication(
            &self,
            scalar: P256SecretKey<p256::NistP256>,
            point: P256PublicKey<p256::NistP256>,
        ) -> Result<p256::elliptic_curve::PublicKey<p256::NistP256>, p256::elliptic_curve::Error>
        {
            let non_zero_scalar: NonZeroScalar<p256::NistP256> = scalar.to_nonzero_scalar();
            let non_identity_point: NonIdentity<AffinePoint<p256::NistP256>> =
                point.to_nonidentity();
            let result_projective_point = non_identity_point.mul(*non_zero_scalar);
            return P256PublicKey::from_affine(result_projective_point.to_affine());
        }

        /*
           @brief: This function computes the private/public ecc key pair of the GW
           @return: the compressed public key of the GW to be sent to the AS
        */
        pub fn generate_ecc_keys(&mut self) -> Box<[u8]> {
            self.private_key = Some(P256SecretKey::random(&mut OsRng));
            self.public_key = Some(self.private_key.clone().unwrap().public_key());
            // Get sec1 bytes of Public Key (TO SEND TO AS)
            self.compressed_public_key = Some(self.public_key.clone().unwrap().to_sec1_bytes());

            return self.compressed_public_key.clone().unwrap();
        }

        /*
           @brief: This function stores the public info of a dev and computes the g_gw_ed to send to the AS
           @param dev_eui: the dev_eui of the device
           @param dev_addr: the dev_addr of the device
           @param g_as_ed_compressed: the compressed g_as_ed computed by the AS
           @param dev_public_key_compressed: the compressed public key of the device
           @return: the g_gw_ed to send to the AS
        */
        pub fn handle_ed_pub_info(
            &mut self,
            dev_eui: String,
            dev_addr: String,
            g_as_ed_compressed: Vec<u8>,
            dev_public_key_compressed: Vec<u8>,
        ) -> Vec<u8> {
            // GET g_as_ed
            let g_as_ed_result: Result<P256PublicKey<p256::NistP256>, p256::elliptic_curve::Error> =
                P256PublicKey::from_sec1_bytes(&g_as_ed_compressed);
            let g_as_ed: P256PublicKey<p256::NistP256>;
            match g_as_ed_result {
                Ok(x) => {
                    g_as_ed = x;
                }
                Err(e) => {
                    println!("Error: {:?}", e);
                    return vec![];
                }
            };

            // Get Device public key
            let dev_public_key_result: Result<
                P256PublicKey<p256::NistP256>,
                p256::elliptic_curve::Error,
            > = P256PublicKey::from_sec1_bytes(&dev_public_key_compressed);
            let dev_public_key: P256PublicKey<p256::NistP256>;
            match dev_public_key_result {
                Ok(x) => {
                    dev_public_key = x;
                }
                Err(e) => {
                    println!("Error: {:?}", e);
                    return vec![];
                }
            };

            // Compute the Edge Session Key
            let edge_s_key_pub_key: P256PublicKey<p256::NistP256> = self
                .scalar_point_multiplication(self.private_key.clone().unwrap(), g_as_ed)
                .unwrap();
            let edge_s_key = edge_s_key_pub_key.as_affine().x();
            let edge_s_key_bytes: Vec<u8> = edge_s_key.to_vec();

            // Compute Edge Session Integrity Key
            let mut edge_s_key_int_bytes_before_hash = edge_s_key_bytes.clone();
            edge_s_key_int_bytes_before_hash.insert(0, 0);
            let edge_s_int_key_hash_result = Sha256::digest(edge_s_key_int_bytes_before_hash);
            let edge_s_int_key_bytes: [u8; 16] =
                edge_s_int_key_hash_result[0..16].try_into().unwrap();
            let edge_s_int_key = AES128::from(edge_s_int_key_bytes);

            // Compute Edge Session Encryption Key
            let mut edge_s_key_enc_bytes_before_hash = edge_s_key_bytes.clone();
            edge_s_key_enc_bytes_before_hash.insert(0, 1);
            let edge_s_enc_key_hash_result = Sha256::digest(edge_s_key_enc_bytes_before_hash);
            let edge_s_enc_key_bytes: [u8; 16] =
                edge_s_enc_key_hash_result[0..16].try_into().unwrap();
            let edge_s_enc_key = AES128::from(edge_s_enc_key_bytes);

            // Add Info to dev info struct
            let mut dev_info_found = false;
            for dev_info in self.active_directory.iter_mut() {
                if dev_info.dev_eui == dev_eui {
                    // if dev_info.dev_addr == dev_addr {
                    dev_info.dev_addr = dev_addr.clone();
                    dev_info.dev_public_key = dev_public_key.clone();
                    dev_info.edge_s_enc_key = edge_s_enc_key;
                    dev_info.edge_s_int_key = edge_s_int_key;
                    dev_info_found = true;
                    break;
                }
            }
            if !dev_info_found {
                let new_dev_info: DevInfo = DevInfo {
                    dev_eui: dev_eui.clone(),
                    dev_addr: dev_addr.clone(),
                    dev_public_key: dev_public_key.clone(),
                    edge_s_enc_key: edge_s_enc_key,
                    edge_s_int_key: edge_s_int_key,
                    values: Vec::new(),
                    _fcnts: Vec::new(),
                };
                self.active_directory.push(new_dev_info);
            }
            println!("Added dev addr: {:?} to active directory.", dev_addr);

            let g_gw_ed = self
                .scalar_point_multiplication(self.private_key.clone().unwrap(), dev_public_key)
                .unwrap();
            return g_gw_ed.to_sec1_bytes().to_vec();
        }

        /*
            @brief: This function checks if the device is in the active directory
            @param dev_addr: the dev_addr of the device
            @return: true if the device is in the active directory, false otherwise
        */
        pub fn check_e2ed_enabled(&self, dev_addr: String) -> bool {
            for dev_info in self.active_directory.iter() {
                if dev_info.dev_addr == dev_addr {
                    return true;
                }
            }
            return false;
        }

        pub fn get_json_mqtt_payload(
            &mut self,
            dev_addr: String,
            fcnt: u16,
            phy: EncryptedDataPayload<Vec<u8>, DefaultFactory>,
            packet: &RxpkContent,
            gwmac: String,
        ) -> Option<MqttJson> {
            let dev_info: &mut DevInfo;

            for dev_info_iter in self.active_directory.iter_mut() {
                if dev_info_iter.dev_addr == dev_addr {
                    dev_info = dev_info_iter;
                    // dev_info_found = true;
                    // GET KEYS
                    let edge_s_enc_key: AES128 = dev_info.edge_s_enc_key.clone();
                    let edge_s_int_key: AES128 = dev_info.edge_s_int_key.clone();
                    let decrypted_data_payload = phy
                        .decrypt(Some(&edge_s_int_key), Some(&edge_s_enc_key), fcnt.into())
                        .unwrap();

                    let frame_payload_result = decrypted_data_payload.frm_payload().unwrap();
                    match frame_payload_result {
                        lorawan_encoding::parser::FRMPayload::Data(frame_payload) => {
                            return Some(MqttJson {
                                dev_eui: dev_info_iter.dev_eui.clone(),
                                dev_addr: dev_info_iter.dev_addr.clone(),
                                fcnt: fcnt,
                                timestamp: packet.tmst,
                                frequency: packet.freq,
                                data_rate: packet.datr.clone(),
                                coding_rate: packet.codr.clone(),
                                gtw_id: gwmac,
                                gtw_channel: packet.chan.unwrap(),
                                gtw_rssi: packet.rssi.unwrap(),
                                gtw_snr: packet.lsnr.unwrap(),
                                payload: base64::encode(frame_payload),
                            });
                        }
                        _ => {
                            println!("Failed to decrypt packet");
                            return None;
                        }
                    }
                }
            }
            return None;
        }

        /*
           @brief: This function processes the frame
           @param dev_addr: the dev_addr of the device
           @param fcnt: the frame counter
           @param phy: the encrypted data payload
           @return: None if aggregated function not defined or device not found (should never happen), an AggregatedResult Structure if success.
           @status_code: 0 if aggregated result if returned and need to send it to DM, -1 otherwise.
        */
        pub fn _process_frame(
            &mut self,
            dev_addr: String,
            fcnt: u16,
            phy: EncryptedDataPayload<Vec<u8>, DefaultFactory>,
        ) -> Option<_ProcessedFrameResult> {
            let dev_info: &mut DevInfo;
            for dev_info_iter in self.active_directory.iter_mut() {
                if dev_info_iter.dev_addr == dev_addr {
                    dev_info = dev_info_iter;
                    // dev_info_found = true;
                    // GET KEYS
                    let edge_s_enc_key: AES128 = dev_info.edge_s_enc_key.clone();
                    let edge_s_int_key: AES128 = dev_info.edge_s_int_key.clone();
                    let decrypted_data_payload = phy
                        .decrypt(Some(&edge_s_int_key), Some(&edge_s_enc_key), fcnt.into())
                        .unwrap();

                    let frame_payload_result = decrypted_data_payload.frm_payload().unwrap();
                    match frame_payload_result {
                        lorawan_encoding::parser::FRMPayload::Data(frame_payload) => {
                            let frame_payload_vec: Vec<u8> = frame_payload.to_vec();
                            let frame_payload_str = String::from_utf8(frame_payload_vec.clone())
                                .expect("Failed to Decode Frame Payload");
                            let timetag: u64 = frame_payload_str.parse().unwrap();
                            let last_index = frame_payload_vec.len() - 1;
                            let sensor_value: i64 = frame_payload_vec[last_index].into();
                            dev_info.values.push(sensor_value);
                            dev_info._fcnts.push(fcnt as u64);
                            if dev_info.values.len() >= self.window_size {
                                let aggregation_result: i64;
                                let mut status_code: i64 = 0;
                                match self.aggregation_function {
                                    i if i == AVG_ID => {
                                        if dev_info.values.len() == 0 {
                                            aggregation_result = 0;
                                            status_code = -1;
                                        } else {
                                            let sum: i64 = dev_info.values.iter().sum();
                                            aggregation_result =
                                                sum / (dev_info.values.len() as i64);
                                        }
                                    }
                                    i if i == SUM_ID => {
                                        if dev_info.values.len() == 0 {
                                            aggregation_result = 0;
                                            status_code = -1;
                                        } else {
                                            aggregation_result = dev_info.values.iter().sum();
                                        }
                                    }
                                    i if i == MIN_ID => {
                                        if dev_info.values.len() == 0 {
                                            aggregation_result = 0;
                                            status_code = -1;
                                        } else {
                                            aggregation_result =
                                                *dev_info.values.iter().min().unwrap();
                                        }
                                    }
                                    i if i == MAX_ID => {
                                        if dev_info.values.len() == 0 {
                                            aggregation_result = 0;
                                            status_code = -1;
                                        } else {
                                            aggregation_result =
                                                *dev_info.values.iter().max().unwrap();
                                        }
                                    }
                                    _ => {
                                        println!("Aggregation function not supported!");
                                        return None;
                                    }
                                }
                                // println!("\n\nValues: {:?}", dev_info.values);
                                // println!("Aggregation result: {:?}\n\n", aggregation_result);
                                let fncts: Vec<u64> = dev_info._fcnts.clone();
                                dev_info.values = Vec::new();
                                dev_info._fcnts = Vec::new();
                                let aggregation_result = _AggregationResult {
                                    status_code: status_code,
                                    dev_eui: dev_info.dev_eui.clone(),
                                    dev_addr: dev_info.dev_addr.clone(),
                                    aggregated_data: aggregation_result,
                                    fcnts: fncts,
                                };
                                let return_value: _ProcessedFrameResult = _ProcessedFrameResult {
                                    timetag: timetag,
                                    aggregation_option: Some(aggregation_result),
                                };

                                return Some(return_value);
                            } else {
                                return Some(_ProcessedFrameResult {
                                    timetag: timetag,
                                    aggregation_option: None,
                                });
                            }
                        }
                        _ => {
                            println!("Failed to decrypt packet");
                            return None;
                        }
                    };
                }
            }
            return None;
        }

        pub fn update_aggregation_params(
            &mut self,
            aggregation_function: u8,
            window_size: usize,
        ) -> i32 {
            self.aggregation_function = aggregation_function;
            self.window_size = window_size;
            println!(
                "Aggregation function updated to: {:?}",
                aggregation_function
            );
            println!("Window size updated to: {:?}", window_size);
            return 0;
        }

        pub fn remove_e2device(
            &mut self,
            dev_eui: String,
        ) -> crate::e2gw_rpc_server::e2gw_rpc_server::e2gw_rpc_server::E2lData {
            let dev_info: &mut DevInfo;
            println!("\nRemoving device: {:?}", dev_eui);
            for dev_info_iter in self.active_directory.iter_mut() {
                if dev_info_iter.dev_eui == dev_eui {
                    dev_info = dev_info_iter;
                    let aggregation_result: i64;
                    let mut status_code: i64 = 0;
                    match self.aggregation_function {
                        i if i == AVG_ID => {
                            if dev_info.values.len() == 0 {
                                aggregation_result = 0;
                                status_code = -1;
                            } else {
                                let sum: i64 = dev_info.values.iter().sum();
                                aggregation_result = sum / (dev_info.values.len() as i64);
                            }
                        }
                        i if i == SUM_ID => {
                            if dev_info.values.len() == 0 {
                                aggregation_result = 0;
                                status_code = -1;
                            } else {
                                aggregation_result = dev_info.values.iter().sum();
                            }
                        }
                        i if i == MIN_ID => {
                            if dev_info.values.len() == 0 {
                                aggregation_result = 0;
                                status_code = -1;
                            } else {
                                aggregation_result = *dev_info.values.iter().min().unwrap();
                            }
                        }
                        i if i == MAX_ID => {
                            if dev_info.values.len() == 0 {
                                aggregation_result = 0;
                                status_code = -1;
                            } else {
                                aggregation_result = *dev_info.values.iter().max().unwrap();
                            }
                        }
                        _ => {
                            println!("Aggregation function not supported!");
                            // get epoch time
                            let start = SystemTime::now();
                            let since_the_epoch = start
                                .duration_since(UNIX_EPOCH)
                                .expect("Time went backwards");
                            let response =
                                crate::e2gw_rpc_server::e2gw_rpc_server::e2gw_rpc_server::E2lData {
                                    status_code: 0,
                                    dev_eui: dev_info.dev_eui.clone(),
                                    dev_addr: dev_info.dev_addr.clone(),
                                    aggregated_data: 0,
                                    aggregated_data_num: 0,
                                    timetag: since_the_epoch.as_millis() as u64,
                                };
                            let index = self
                                .active_directory
                                .iter()
                                .position(|x| *x.dev_eui == dev_eui)
                                .unwrap();
                            self.active_directory.remove(index);
                            return response;
                        }
                    }

                    // get epoch time
                    let start = SystemTime::now();
                    let since_the_epoch = start
                        .duration_since(UNIX_EPOCH)
                        .expect("Time went backwards");
                    let response =
                        crate::e2gw_rpc_server::e2gw_rpc_server::e2gw_rpc_server::E2lData {
                            status_code: status_code,
                            dev_eui: dev_info.dev_eui.clone(),
                            dev_addr: dev_info.dev_addr.clone(),
                            aggregated_data: aggregation_result,
                            aggregated_data_num: dev_info.values.len() as u64,
                            timetag: since_the_epoch.as_millis() as u64,
                        };
                    let index = self
                        .active_directory
                        .iter()
                        .position(|x| *x.dev_eui == dev_eui)
                        .unwrap();
                    self.active_directory.remove(index);
                    return response;
                }
            }
            let response = crate::e2gw_rpc_server::e2gw_rpc_server::e2gw_rpc_server::E2lData {
                status_code: -1,
                dev_eui: "".to_string(),
                dev_addr: "".to_string(),
                aggregated_data: 0,
                aggregated_data_num: 0,
                timetag: 0,
            };
            return response;
        }

        pub fn add_devices(
            &mut self,
            device_list: Vec<crate::e2gw_rpc_server::e2gw_rpc_server::e2gw_rpc_server::Device>,
        ) -> crate::e2gw_rpc_server::e2gw_rpc_server::e2gw_rpc_server::GwResponse {
            let device_list_len = device_list.len();
            for device in device_list {
                // Create fake priv pub device key
                let dev_fake_private_key = Some(P256SecretKey::random(&mut OsRng));
                let dev_fake_public_key =
                    Some(dev_fake_private_key.clone().unwrap().public_key()).unwrap();
                let dev_eui = device.dev_eui;
                let dev_addr = device.dev_addr;
                let edge_s_enc_key_vec = device.edge_s_enc_key;
                let edge_s_enc_key_bytes: [u8; 16] = edge_s_enc_key_vec.try_into().unwrap();
                let edge_s_enc_key = AES128::from(edge_s_enc_key_bytes.clone());

                let edge_s_int_key_vec = device.edge_s_int_key;
                let edge_s_int_key_bytes: [u8; 16] = edge_s_int_key_vec.try_into().unwrap();
                let edge_s_int_key = AES128::from(edge_s_int_key_bytes.clone());

                // Create DevInfo struct and add to active directory
                let new_dev_info: DevInfo = DevInfo {
                    dev_eui: dev_eui,
                    dev_addr: dev_addr,
                    dev_public_key: dev_fake_public_key,
                    edge_s_enc_key: edge_s_enc_key,
                    edge_s_int_key: edge_s_int_key,
                    values: Vec::new(),
                    _fcnts: Vec::new(),
                };
                self.active_directory.push(new_dev_info);
            }

            let response = crate::e2gw_rpc_server::e2gw_rpc_server::e2gw_rpc_server::GwResponse {
                status_code: 0,
                message: "Devices added".to_string(),
            };
            println!("ADDED {} DEVICES", device_list_len);
            return response;
        }
    }
}

pub static mut E2L_CRYPTO: E2LCrypto = E2LCrypto {
    private_key: None,
    public_key: None,
    compressed_public_key: None,
    active_directory: Vec::new(),
    aggregation_function: AVG_ID,
    window_size: 5,
    is_active: true,
};
