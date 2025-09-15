use sysinfo::{NetworkExt, NetworksExt, System, SystemExt};

pub trait NtwkStats {
    /// Returns the total network upload usage in kilobits (KB).
    /// # Arguments
    ///
    /// * `sys` - A reference to a [`System`] instance from the `sysinfo` crate,
    ///           which should already be refreshed using `.refresh_networks()`
    ///           or `.refresh_all()` to get up-to-date network statistics.
    ///
    /// # Returns
    ///
    /// # Example
    ///
    /// ```
    /// use sysinfo::{System, SystemExt, NetworkExt};
    ///
    /// let mut sys = System::new_all();
    /// sys.refresh_networks();
    ///
    /// let up_kb = get_ntwk_up(&sys);
    /// println!("Total upload: {} KB", up_kb);
    /// ```
    ///
    fn get_ntwk_up(&mut self) -> i32;

    /// Returns the total network download usage in kilobits (KB).
    /// # Arguments
    ///
    /// * `sys` - A reference to a [`System`] instance from the `sysinfo` crate,
    ///           which should already be refreshed using `.refresh_networks()`
    ///           or `.refresh_all()` to get up-to-date network statistics.
    ///
    /// # Returns
    ///
    /// # Example
    ///
    /// ```
    /// use sysinfo::{System, SystemExt, NetworkExt};
    ///
    /// let mut sys = System::new_all();
    /// sys.refresh_networks();
    ///
    /// let down_kb = get_ntwk_dwn(&sys);
    /// println!("Total download: {} KB", down_kb);
    /// ```
    ///
    fn get_ntwk_down(&mut self) -> i32;

    /// Returns the percentage of used swap memory.
    /// 
    /// # Arguments
    /// 
    /// * `self` - A mutable reference to a [`System`] instance from the `sysinfo` crate,
    ///           which should have up-to-date memory information. Ensure you call
    ///           `.refresh_memory()` or `.refresh_all()` before using this function.
    /// 
    /// # Returns
    /// 
    /// A `u64` value representing the percentage of used swap memory (0–100).
    /// If total swap is zero, this function will panic due to division by zero.
    /// 
    /// # Example
    /// 
    /// ```
    /// use sysinfo::{System, SystemExt};
    /// 
    /// let mut sys = System::new_all();
    /// sys.refresh_memory();
    /// 
    /// let swap_percentage = sys.swap_used();
    /// println!("Swap used: {}%", swap_percentage);
    /// ```
    fn swap_used(&mut self) -> u64;

}

impl NtwkStats for System {
    fn get_ntwk_up(&mut self) -> i32 {
        self.refresh_networks();
        self.networks_mut()
            .iter()
            .map(|(_, iface)| iface.transmitted() as i32)
            .sum::<i32>() / 128
    }
    fn get_ntwk_down(&mut self) -> i32 {
        self.refresh_networks();
        self.networks_mut()
            .iter()
            .map(|(_, iface)| iface.received() as i32)
            .sum::<i32>() / 128
    }
    fn swap_used(&mut self) -> u64 {
        (self.used_swap() as u64 * 100) / (self.total_swap() as u64)
    }
}