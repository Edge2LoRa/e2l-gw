static _AVG_ID: u8 = 1;
static _SUM_ID: u8 = 2;
static _MIN_ID: u8 = 3;
static _MAX_ID: u8 = 4;
pub(crate) mod e2l_crypto {
    use base64::{engine::general_purpose, Engine as _};
    // MUTEX
    use std::sync::{Arc, Mutex, MutexGuard};

    // Crypto
    extern crate p256;
    extern crate serde_json;

    use gethostname::gethostname;
    use std::ops::Mul;

    use lorawan_encoding::default_crypto::DefaultFactory;
    use lorawan_encoding::keys::AES128;
    use lorawan_encoding::parser::{AsPhyPayloadBytes, EncryptedDataPayload};
    use p256::elliptic_curve::point::AffineCoordinates;
    use p256::elliptic_curve::point::NonIdentity;
    use p256::elliptic_curve::rand_core::OsRng;
    use p256::elliptic_curve::AffinePoint;
    use p256::elliptic_curve::NonZeroScalar;
    use p256::elliptic_curve::PublicKey as P256PublicKey;
    use p256::elliptic_curve::SecretKey as P256SecretKey;
    use sha2::Digest;
    use sha2::Sha256;

    use crate::e2gw_rpc_server::e2gw_rpc_server::e2gw_rpc_server::{Device, E2lData, GwResponse};
    use crate::lorawan_structs::lorawan_structs::lora_structs::RxpkContent;
    use crate::mqtt_client::mqtt_structs::mqtt_structs::{MqttJson, UnassociatedMqttJson};

    // ACTIVE DIRECTORY
    use crate::e2l_active_directory::e2l_active_directory::e2l_active_directory::{
        AssociatedDevInfo, E2LActiveDirectory, UnassociatedDevInfo,
    };

    pub struct E2LCrypto {
        pub gw_id: String,
        pub private_key: Option<P256SecretKey<p256::NistP256>>,
        pub public_key: Option<P256PublicKey<p256::NistP256>>,
        pub compressed_public_key: Option<Box<[u8]>>,
        pub active_directory_mutex: Arc<Mutex<E2LActiveDirectory>>,
        is_active: Arc<Mutex<bool>>,
    }

    struct KeyInfo {
        pub private_key: Option<P256SecretKey<p256::NistP256>>,
        pub public_key: Option<P256PublicKey<p256::NistP256>>,
        pub compressed_public_key: Option<Box<[u8]>>,
    }

    impl E2LCrypto {
        /*
           @brief: This function computes the private/public ecc key pair of the GW
           @return: the compressed public key of the GW to be sent to the AS
        */
        fn generate_ecc_keys() -> KeyInfo {
            let private_key = Some(P256SecretKey::random(&mut OsRng));
            let public_key = Some(private_key.clone().unwrap().public_key());
            // Get sec1 bytes of Public Key (TO SEND TO AS)
            let compressed_public_key = Some(public_key.clone().unwrap().to_sec1_bytes());

            return KeyInfo {
                private_key,
                public_key,
                compressed_public_key,
            };
        }

        /*
           @brief: This function multiplies a scalar with a point on the curve
           @param scalar: the scalar to multiply as private key
           @param point: the point to multiply as public key
           @return: the result of the scalar multiplication
        */
        fn scalar_point_multiplication(
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
           @brief: This function return a new E2LCrypto object
        */
        pub fn new(hostname: String) -> Self {
            let key_info = Self::generate_ecc_keys();
            let return_value = E2LCrypto {
                gw_id: hostname,
                private_key: key_info.private_key,
                public_key: key_info.public_key,
                compressed_public_key: key_info.compressed_public_key,
                active_directory_mutex: Arc::new(Mutex::new(E2LActiveDirectory::new())),
                is_active: Arc::new(Mutex::new(false)),
            };

            return return_value;
        }

        pub fn set_active(&self, is_active: bool) {
            let mut aux = self.is_active.lock().expect("Could not lock");
            *aux = is_active;
            std::mem::drop(aux);
        }

        pub fn is_active(&self) -> bool {
            let mutex = self.is_active.lock().expect("Could not lock!");
            let is_active = *mutex;
            std::mem::drop(mutex);
            return is_active;
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
            &self,
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
            let edge_s_key_pub_key: P256PublicKey<p256::NistP256> =
                Self::scalar_point_multiplication(self.private_key.clone().unwrap(), g_as_ed)
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
            let mut active_directory: MutexGuard<E2LActiveDirectory> =
                self.active_directory_mutex.lock().expect("Could not lock");
            active_directory.add_associated_dev(
                dev_eui.clone(),
                dev_addr.clone(),
                dev_public_key.clone(),
                edge_s_enc_key,
                edge_s_int_key,
            );
            std::mem::drop(active_directory);
            println!("Added dev addr: {:?} to active directory.", dev_addr);

            let g_gw_ed = Self::scalar_point_multiplication(
                self.private_key.clone().unwrap(),
                dev_public_key,
            )
            .unwrap();
            return g_gw_ed.to_sec1_bytes().to_vec();
        }

        /*
            @brief: This function checks if the device is in the active directory
            @param dev_addr: the dev_addr of the device
            @return: true if the device is in the active directory, false otherwise
        */
        pub fn check_e2ed_enabled(&self, dev_addr: String) -> bool {
            let active_directory: MutexGuard<E2LActiveDirectory> =
                self.active_directory_mutex.lock().expect("Could not lock");
            let ret = active_directory.is_associated_dev(&dev_addr);
            std::mem::drop(active_directory);
            return ret;
        }

        /*
           @brief: This function handles the encrypted data payload of a packet
           @param dev_addr: the dev_addr of the device
           @param fcnt: the fcnt of the packet
           @param phy: the encrypted data payload of the packet
           @param packet: the RxpkContent of the packet
           @param gwmac: the gwmac of the packet
           @return: the MqttJson of the packet
           @note: the MqttJson is sent to the MQTT broker
        */
        pub fn get_json_mqtt_payload(
            &self,
            dev_addr: String,
            fcnt: u16,
            phy: EncryptedDataPayload<Vec<u8>, DefaultFactory>,
            packet: &RxpkContent,
            gwmac: String,
        ) -> Option<MqttJson> {
            let active_directory: MutexGuard<E2LActiveDirectory> =
                self.active_directory_mutex.lock().expect("Could not lock!");
            let dev_info_option: Option<&AssociatedDevInfo> =
                active_directory.get_associated_dev(&dev_addr.clone());
            match dev_info_option {
                Some(dev_info) => {
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
                                dev_eui: dev_info.dev_eui.clone(),
                                dev_addr: dev_info.dev_addr.clone(),
                                fcnt: fcnt,
                                timestamp: packet.time.clone().unwrap_or("".to_string()),
                                frequency: packet.freq,
                                data_rate: packet.datr.clone(),
                                coding_rate: packet.codr.clone(),
                                gtw_id: gwmac,
                                gtw_channel: packet.chan.unwrap(),
                                gtw_rssi: packet.rssi.unwrap(),
                                gtw_snr: packet.lsnr.unwrap(),
                                payload: general_purpose::STANDARD.encode(frame_payload),
                            });
                        }
                        _ => {
                            println!("Failed to decrypt packet");
                            return None;
                        }
                    }
                }
                None => return None,
            }
        }

        pub fn get_unassociated_json_mqtt_payload(
            &self,
            dev_addr: String,
            fcnt: u16,
            phy: EncryptedDataPayload<Vec<u8>, DefaultFactory>,
            packet: &RxpkContent,
            gwmac: String,
        ) -> Option<UnassociatedMqttJson> {
            let active_directory: MutexGuard<E2LActiveDirectory> =
                self.active_directory_mutex.lock().unwrap();
            let dev_info_option: Option<&UnassociatedDevInfo> =
                active_directory.get_unassociated_dev(&dev_addr.clone());
            match dev_info_option {
                Some(dev_info) => {
                    // GET KEYS
                    return Some(UnassociatedMqttJson {
                        dev_eui: dev_info.dev_eui.clone(),
                        dev_addr: dev_info.dev_addr.clone(),
                        gw_id: dev_info.e2gw_id.clone(),
                        fcnt: fcnt,
                        timestamp: packet.tmst,
                        frequency: packet.freq,
                        data_rate: packet.datr.clone(),
                        coding_rate: packet.codr.clone(),
                        gtw_id: gwmac,
                        gtw_channel: packet.chan.unwrap(),
                        gtw_rssi: packet.rssi.unwrap(),
                        gtw_snr: packet.lsnr.unwrap(),
                        encrypted_payload: general_purpose::STANDARD.encode(phy.as_bytes()),
                    });
                }
                None => return None,
            }
        }

        pub fn remove_e2device(&self, dev_addr: String) -> E2lData {
            let mut active_directory: MutexGuard<E2LActiveDirectory> =
                self.active_directory_mutex.lock().unwrap();
            if active_directory.is_associated_dev(&dev_addr.clone()) {
                active_directory.remove_associated_dev(&dev_addr.clone());
            } else {
                active_directory.remove_unassociated_dev(&dev_addr.clone());
            }
            std::mem::drop(active_directory);
            println!("Device removed: {:?}", dev_addr);
            let response = E2lData {
                status_code: -1,
                dev_eui: "".to_string(),
                dev_addr: "".to_string(),
                aggregated_data: 0,
                aggregated_data_num: 0,
                timetag: 0,
            };
            return response;
        }

        pub fn add_devices(&self, device_list: Vec<Device>) -> GwResponse {
            let device_list_len = device_list.len();
            let mut active_directory: MutexGuard<E2LActiveDirectory> =
                self.active_directory_mutex.lock().unwrap();
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

                let assigned_gw = device.assigned_gw;
                if assigned_gw != self.gw_id {
                    active_directory.add_unassociated_dev(dev_eui, dev_addr, assigned_gw);
                } else {
                    active_directory.add_associated_dev(
                        dev_eui,
                        dev_addr,
                        dev_fake_public_key,
                        edge_s_enc_key,
                        edge_s_int_key,
                    );
                }
            }
            std::mem::drop(active_directory);

            let response = GwResponse {
                status_code: 0,
                message: "Devices added".to_string(),
            };
            println!("INFO: ADDED {} DEVICES", device_list_len);
            return response;
        }
    }

    impl Default for E2LCrypto {
        fn default() -> Self {
            Self::new(gethostname().into_string().unwrap())
        }
    }
}
