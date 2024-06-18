pub(crate) mod e2l_active_directory {
    extern crate p256;
    use lorawan_encoding::keys::AES128;
    use p256::elliptic_curve::PublicKey as P256PublicKey;
    use std::collections::HashMap;

    #[derive(Clone)]
    pub struct UnassociatedDevInfo {
        pub dev_eui: String,
        pub dev_addr: String,
        pub e2gw_id: String,
    }

    #[derive(Clone)]
    pub struct AssociatedDevInfo {
        pub dev_eui: String,
        pub dev_addr: String,
        pub dev_public_key: P256PublicKey<p256::NistP256>,
        pub edge_s_enc_key: AES128,
        pub edge_s_int_key: AES128,
        pub fcnts: Vec<u16>,
    }

    pub struct E2LActiveDirectory {
        unassociated_dev_info: HashMap<String, UnassociatedDevInfo>,
        associated_dev_info: HashMap<String, AssociatedDevInfo>,
    }

    impl Default for E2LActiveDirectory {
        fn default() -> Self {
            Self::new()
        }
    }

    impl E2LActiveDirectory {
        pub fn new() -> Self {
            E2LActiveDirectory {
                unassociated_dev_info: HashMap::new(),
                associated_dev_info: HashMap::new(),
            }
        }

        /*
         * @brief: Add unassociated device to the active directory.
         * @param: dev_eui: device EUI.
         * @param: dev_addr: device address.
         * @param: e2gw_id: E2GW ID.
         * @return: None.
         */
        pub fn add_unassociated_dev(&mut self, dev_eui: String, dev_addr: String, e2gw_id: String) {
            self.unassociated_dev_info.insert(
                dev_addr.clone(),
                UnassociatedDevInfo {
                    dev_eui,
                    dev_addr,
                    e2gw_id,
                },
            );
        }

        /*
         * @brief: Add associated device to the active directory.
         * @param: dev_eui: device EUI.
         * @param: dev_addr: device address.
         * @param: dev_public_key: device public key.
         * @param: edge_s_enc_key: edge session encryption key.
         * @param: edge_s_int_key: edge session integrity key.
         */
        pub fn add_associated_dev(
            &mut self,
            dev_eui: String,
            dev_addr: String,
            dev_public_key: P256PublicKey<p256::NistP256>,
            edge_s_enc_key: AES128,
            edge_s_int_key: AES128,
        ) {
            let dev_info_option = self.associated_dev_info.get(&dev_addr);
            let fncts: Vec<u16>;
            match dev_info_option {
                Some(dev_info) => {
                    fncts = dev_info.fcnts.clone();
                }
                None => {
                    fncts = Vec::new();
                }
            }
            self.associated_dev_info.insert(
                dev_addr.clone(),
                AssociatedDevInfo {
                    dev_eui,
                    dev_addr,
                    dev_public_key,
                    edge_s_enc_key,
                    edge_s_int_key,
                    fcnts: fncts,
                },
            );
        }

        /*
         * @brief: Get unassociated device from the active directory.
         * @param: dev_addr: device address.
         * @return: Unassociated device info.
         */
        pub fn get_unassociated_dev(&self, dev_addr: &str) -> Option<UnassociatedDevInfo> {
            let res = self.unassociated_dev_info.get(dev_addr);
            match res {
                Some(dev_info) => Some(dev_info.clone()),
                None => None,
            }
        }

        /*
         * @brief: Get associated device from the active directory.
         * @param: dev_addr: device address.
         * @return: Associated device info.
         */
        pub fn get_associated_dev(&self, dev_addr: &str) -> Option<AssociatedDevInfo> {
            let res = self.associated_dev_info.get(dev_addr);
            match res {
                Some(dev_info) => Some(dev_info.clone()),
                None => None,
            }
        }

        /*
         * @brief: Remove unassociated device from the active directory.
         * @param: dev_addr: device address.
         * @return: None.
         */
        pub fn remove_unassociated_dev(&mut self, dev_addr: &str) {
            self.unassociated_dev_info.remove(dev_addr);
        }

        /*
         * @brief: Remove associated device from the active directory.
         * @param: dev_addr: device address.
         * @return: None.
         */
        pub fn remove_associated_dev(&mut self, dev_addr: &str) {
            self.associated_dev_info.remove(dev_addr);
        }

        /*
         * @brief: Check if the device is associated.
         * @param: dev_addr: device address.
         * @return: True if the device is associated, false otherwise.
         */
        pub fn is_associated_dev(&self, dev_addr: &str) -> bool {
            self.associated_dev_info.contains_key(dev_addr)
        }

        /*
         * @brief: Add a fcnt to device array.
         * @param: dev_addr: device address.
         * @param: fcnt: fcnt.
         */
        pub fn add_fcnt(&mut self, dev_addr: &str, fcnt: u16) {
            if let Some(dev_info) = self.associated_dev_info.get_mut(dev_addr) {
                dev_info.fcnts.push(fcnt);
            }
        }

        /*
         * @brief: Clear the active directory.
         * #return: None.
         */
        fn _clear(&mut self) {
            self.unassociated_dev_info.clear();
            self.associated_dev_info.clear();
        }
    }
}
