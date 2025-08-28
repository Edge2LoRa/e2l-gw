pub(crate) mod e2l_end_device{
    use serde_derive::Deserialize;
    use serde_derive::Serialize;
    use crate::lorawan_structs::lorawan_structs::lora_structs::RxpkContent;

    #[derive(Debug)]
    pub struct DevicePks{
     pub dev_eui:String,
     pub dev_addr:String,
     pub rxpk:Vec<RxpkContent>
    }
    
    #[derive(Debug, Serialize, Deserialize)]
    pub struct DeviceStats{
        pub dev_eui: String,
        pub dev_addr: String,
        pub avg_rssi: f64,
        pub avg_snr: f64,
        pub avg_payload_size:f64
    }
    impl DevicePks {
        pub fn avg_rssi(&self) -> Option<f64> {
            let (sum, count) = self.rxpk.iter()
                .filter_map(|p| p.rssi)
                .map(|rssi| rssi as f64)
                .fold((0.0, 0), |(sum, count), rssi| (sum + rssi, count + 1));

            if count == 0 {
                None
            } else {
                Some(sum / count as f64)
            }
        }

        pub fn avg_snr(&self) -> Option<f64> {
            let (sum, count) = self.rxpk.iter()
                .filter_map(|p| p.lsnr)
                .map(|lsnr| lsnr as f64)
                .fold((0.0, 0), |(sum, count), lsnr| (sum + lsnr, count + 1));

            if count == 0 {
                None
            } else {
                Some(sum / count as f64)
            }
        }

        pub fn avg_payload_size(&self) -> Option<f64> {
            let count = self.rxpk.len();
            if count == 0 {
                return None;
            }
            let total: f64 = self.rxpk.iter().map(|p| p.size as f64).sum();
            Some(total / count as f64)
        }
    }

}

