pub(crate) mod e2l_end_device{
    use std::sync::{Arc, Mutex};
    use std::collections::HashSet;
    use ordered_float::OrderedFloat;
    use std::collections::HashMap;




    use serde_derive::Deserialize;
    use serde_derive::Serialize;
    use crate::lorawan_structs::lora_structs::RxpkContent;

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

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct DevicePks{
        pub dev_eui:String,
        pub dev_addr:String,
        pub rxpk:Vec<RxpkContent>,
        pub modu_set: HashSet<String>,
        pub freq_set: HashSet<OrderedFloat<f32>>,
        pub chan_set: HashSet<u32>,
        pub sf_set: HashSet<u8>,
        pub bw_set: HashSet<u32>
    }
    
    #[derive(Debug, Clone, Default)]
    //To access a hash map inside mutex
    pub struct DeviceMap {
        pub inner: Arc<Mutex<HashMap<String, DevicePks>>>,
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct DeviceStats{
        pub dev_eui: String,
        pub frames: FrameCounters,
        pub fcnt: u16,  
        pub dev_addr: String,
        pub avg_rssi: f64,
        pub avg_snr: f64,
        pub avg_payload_size:f64,
        pub modu: HashSet<String>,
        pub freq: HashSet<OrderedFloat<f32>>,
        pub chan: HashSet<u32>,
        pub sf: HashSet<u8>,
        pub bw: HashSet<u32>,
    }

    #[derive(Debug, Serialize, Clone)]
    pub struct CombinedStats{
        pub gw_stats: GwStats,
        pub devices_stats: HashMap<String, DeviceStats>
        //Controlling Five frame counters
        /*new_rx_frame
        new_rx_ho_frame
        new_proc_frame
        new_fw_frame
        new_tx_ho_frame*/
    }

    trait  Frames {
       fn get(&self);
       fn reset(&mut self);
        
    }
    impl Frames for CombinedStats {
        fn get(&self) {
            todo!("Implement get() to control frame counters later")
        }
        
        fn reset(&mut self) {
            todo!("Implement reset() to reset frame counters later")
        }
        
    }


    impl DeviceMap {
        pub fn new() -> Self {
            DeviceMap {
                inner: Arc::new(Mutex::new(HashMap::new())),
            }
        }
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

