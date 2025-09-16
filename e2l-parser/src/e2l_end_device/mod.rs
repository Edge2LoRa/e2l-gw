pub(crate) mod e2l_end_device{
    use std::collections::HashSet;
    use ordered_float::OrderedFloat;


    use serde_derive::Deserialize;
    use serde_derive::Serialize;
    use crate::e2l_mqtt_client::e2l_mqtt_client::FrameCounters;
    use crate::lorawan_structs::lora_structs::RxpkContent;

    #[derive(Debug, Serialize, Deserialize)]
    pub struct DevicePks{
     pub dev_eui:String,
     pub dev_addr:String,
     pub rxpk:Vec<RxpkContent>,
     pub modu_set: HashSet<String>,
     pub freq_set: HashSet<OrderedFloat<f32>>,
     pub chan_set: HashSet<Option<u32>>,
     pub sf_set: HashSet<u8>,
     pub bw_set: HashSet<u32>
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct DeviceStats{
        pub dev_eui: String,
        // pub frames: FrameCounters,  
        pub dev_addr: String,
        pub avg_rssi: f64,
        pub avg_snr: f64,
        pub avg_payload_size:f64,
        pub modu: HashSet<String>,
        pub freq: HashSet<OrderedFloat<f32>>,
        pub chan: HashSet<Option<u32>>,
        pub sf: HashSet<u8>,
        pub bw: HashSet<u32>
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
        // The influence of ADR for evaluating the link quality
        pub fn parse_datr(datr: &str) -> Option<(u8, u32)> {
            // Strip "SF" and split by "BW"
            if let Some(datr) = datr.strip_prefix("SF") {
                let parts: Vec<&str> = datr.split("BW").collect();
                if parts.len() == 2 {
                    let sf = parts[0].parse::<u8>().ok()?;
                    let bw = parts[1].parse::<u32>().ok()? * 1000; // Convert kHz to Hz
                    return Some((sf, bw));
                }
            }
            None
        }
    }

}

