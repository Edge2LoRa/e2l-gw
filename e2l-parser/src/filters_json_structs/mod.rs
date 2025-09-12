pub(crate) mod filter_json {
    use serde_derive::Deserialize;
    use serde_derive::Serialize;

    #[derive(Debug, Serialize, Deserialize)]
    pub struct FilterJson {
        pub(crate) dev_addr: Vec<String>,
        pub(crate) dev_addr_intervals: Vec<DevAddrIntervals>,
        pub(crate) dev_eui_intervals: Vec<DevEuiIntervals>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct DevAddrIntervals {
        pub dev_addr_start: String,
        pub dev_addr_end: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct DevEuiIntervals {
        pub dev_eui_start: String,
        pub dev_eui_end: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct EnvVariables {
        pub local_port: String,
        pub remote_port: String,
        pub remote_addr: String,
        pub bind_addr: String,
        pub filters: String,
        pub debug: bool,
    }
}
