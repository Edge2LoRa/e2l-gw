pub(crate) mod e2l_device{
    use serde_derive::Deserialize;
    use serde_derive::Serialize;
    use crate::lorawan_structs::lorawan_structs::lora_structs::RxpkContent;
    
    #[derive(Debug, Serialize, Deserialize)]
    pub struct DeviceStats{
        pub dev_eui: String,
        pub dev_addr: String,
        //LoRa gateways send rxpk as a JSON array
        pub rpxk: Vec<RxpkContent>,
        pub avg_rssi: f32,
        pub avg_snr: f32,
        pub avg_payload_size:f64
    }

    impl DeviceStats {
        pub fn avg_rssi(&self) -> Option<f32> {
           let (sum, count) = self.rpxk.iter()
                .filter_map(|p| p.rssi) 
                .map(|rssi| rssi as f32)
                .fold((0.0, 0), |(sum, count), rssi| (sum + rssi, count + 1));

            if count == 0 {
                None
            } else {
                Some(sum / count as f32)
            }
        }

        pub fn avg_snr(&self) -> Option<f32> {
             let (sum, count) = self.rpxk.iter()
                .filter_map(|p| p.lsnr) 
                .map(|lsnr| lsnr as f32)
                .fold((0.0, 0), |(sum, count), lsnr| (sum + lsnr, count + 1));

            if count == 0 {
                None
            } else {
                Some(sum / count as f32)
            }
        }

        pub fn avg_payload_size(&self) -> Option<f64> {
            let count = self.rpxk.len();
            if count == 0 {
                return None;
            }
            Some(self.rpxk.iter().map(|p| p.size as f64).sum::<f64>() / count as f64)
        }
    }
}

