use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};

pub trait NtwkStats {
    fn get_ntwk_up(&mut self) -> i32;
}

impl NtwkStats for System {
    fn get_ntwk_up(&mut self) -> i32 {
        self.refresh_networks();
        self.networks_mut()
            .iter()
            .map(|(_, iface)| iface.transmitted() as i32)
            .sum::<i32>() / 128
    }
}
