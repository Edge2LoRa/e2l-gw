pub(crate) mod e2l_end_device{
    use std::collections::HashSet;
    use ordered_float::OrderedFloat;
    use std::collections::HashMap;
    use serde_derive::Deserialize;
    use serde_derive::Serialize;

    use crate::lorawan_structs::lora_structs::RxpkContent;
    // One Global variable will be used by multiple threads


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

     #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct GwStats {
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
    impl Default for GwStats {
        fn default() -> Self {
            Self::new()
        }
    }
    impl GwStats {
        pub fn new() -> Self{ GwStats { frame: FrameCounters::default(), mem_available: 0, mem_usage: 0, mem_usage_percentage: 0, swp_usage_percentage: 0, ntwk_down: 0, ntwk_up: 0, cpu_usage: 0.0, cpu_usage_percentage: 0.0 }}
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct DeviceStats{
        pub frames: FrameCounters,
        pub fcnt: u16,  
        pub avg_rssi: f64,
        pub avg_snr: f64,
        pub avg_payload_size:f64,
        pub modu: HashSet<String>,
        pub freq: HashSet<OrderedFloat<f32>>,
        pub chan: HashSet<u32>,
        pub sf: HashSet<String>,
        pub bw: HashSet<u32>,
    }
    impl Default for DeviceStats {
        fn default() -> Self {
            Self::new()
        }
    }

    impl DeviceStats {
        pub fn new() -> Self {
            DeviceStats { frames: FrameCounters::default(), fcnt: 0, avg_rssi: 0.0, avg_snr: 0.0, avg_payload_size: 0.0, modu: HashSet::new(), freq: HashSet::new(), chan: HashSet::new(), sf: HashSet::new(), bw: HashSet::new() }
        }
    }

    #[derive(Debug, Serialize, Clone)]
    pub struct CombinedStats{
        pub gw_stats: GwStats,
        pub devices_stats: HashMap<String, DeviceStats>,

    }
    impl Default for CombinedStats {
        fn default() -> Self {
            Self::new()
        }
    }
    impl CombinedStats {
        pub fn new() -> Self{
            CombinedStats { gw_stats: GwStats::default(), devices_stats: HashMap::new() }
        }
        pub fn record_rx_frame(&mut self, dev_addr: String, packet:RxpkContent){
            self.gw_stats.frame.rx_frames += 1;
            let dev_stats_options= self.devices_stats.get_mut(&dev_addr);
            

            match dev_stats_options {
                Some(dev_stats) => {
                    let rx_frames:f64=dev_stats.frames.rx_frames as f64;
                    //avg pavload
                    let packet_size:f64=packet.size as f64;
                    dev_stats.avg_payload_size=((dev_stats.avg_payload_size * rx_frames) + packet_size)/(rx_frames+1.0);
                    //avg rssi
                    let rssi=packet.rssi.unwrap() as f64;
                    dev_stats.avg_rssi=((dev_stats.avg_rssi * rx_frames) + rssi)/(rx_frames+1.0);
                    //avg snr
                    let snr:f64 =packet.lsnr.unwrap() as f64;
                    dev_stats.avg_snr =((dev_stats.avg_snr * rx_frames) + snr)/(rx_frames+1.0);

                    dev_stats.modu.insert(packet.modu);
                    dev_stats.freq.insert(ordered_float::OrderedFloat(packet.freq));

                    let chan:u32 =packet.chan.unwrap() as u32;
                    dev_stats.chan.insert(chan);

                    dev_stats.sf.insert((&packet.datr[..3]).into());
                    dev_stats.bw.insert(packet.datr[5..].parse::<u32>().unwrap_or(0));

                    dev_stats.frames.rx_frames +=1;
                },
                None => {
                    let mut new_device_stats = DeviceStats::default();

                    new_device_stats.avg_payload_size=packet.size as f64;
                    new_device_stats.avg_rssi=packet.rssi.unwrap() as f64;
                    new_device_stats.avg_snr=packet.lsnr.unwrap() as f64;
                    new_device_stats.modu.insert(packet.modu);
                    new_device_stats.freq.insert(OrderedFloat(packet.freq));
                    new_device_stats.chan.insert(packet.chan.unwrap() as u32);
                    new_device_stats.sf.insert((&packet.datr[..3]).into());
                    new_device_stats.bw.insert(packet.datr[5..].parse::<u32>().unwrap_or(0));

                    new_device_stats.frames.rx_frames +=1;
                    self.devices_stats.insert(dev_addr, new_device_stats);
                },
            }
        }
        pub fn record_rx_ho_frame(&mut self, dev_addr: String) {
            self.gw_stats.frame.rx_ho_frames += 1;
            let dev_stats_options= self.devices_stats.get_mut(&dev_addr);

            match dev_stats_options {
                Some(dev_stats) => {
                    dev_stats.frames.rx_ho_frames +=1;
                },
                None => {
                    let mut new_device_stats = DeviceStats::default();
                    new_device_stats.frames.rx_ho_frames +=1;
                    self.devices_stats.insert(dev_addr, new_device_stats);
                },
            }
        }
        pub fn record_fw_frame(&mut self, dev_addr: String){
            self.gw_stats.frame.fw_frames += 1;
            let dev_stats_options= self.devices_stats.get_mut(&dev_addr);

            match dev_stats_options {
                Some(dev_stats) => {
                    dev_stats.frames.fw_frames +=1;
                },
                None => {
                    let mut new_device_stats = DeviceStats::default();
                    new_device_stats.frames.fw_frames +=1;
                    self.devices_stats.insert(dev_addr, new_device_stats);
                },
            }
        }
        pub fn record_proc_frame(&mut self, dev_addr: String){
            self.gw_stats.frame.proc_frames += 1;
            let dev_stats_options= self.devices_stats.get_mut(&dev_addr);

            match dev_stats_options {
                Some(dev_stats) => {
                    dev_stats.frames.proc_frames +=1;
                },
                None => {
                    let mut new_device_stats = DeviceStats::default();
                    new_device_stats.frames.proc_frames +=1;
                    self.devices_stats.insert(dev_addr, new_device_stats);
                },
            }
        }
        pub fn record_tx_ho_frame(&mut self, dev_addr: String){
            self.gw_stats.frame.tx_ho_frames += 1;
            let dev_stats_options= self.devices_stats.get_mut(&dev_addr);

            match dev_stats_options {
                Some(dev_stats) => {
                    dev_stats.frames.tx_ho_frames +=1;
                },
                None => {
                    let mut new_device_stats = DeviceStats::default();
                    new_device_stats.frames.tx_ho_frames +=1;
                    self.devices_stats.insert(dev_addr, new_device_stats);
                },
            }
        }
        pub fn reset(&mut self){
            self.gw_stats= GwStats::default();
            self.devices_stats=HashMap::new();
        }
    }

}

