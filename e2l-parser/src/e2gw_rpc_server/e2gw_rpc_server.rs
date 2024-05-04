pub(crate) mod e2gw_rpc_server {

    use crate::e2l_crypto::e2l_crypto::e2l_crypto::E2LCrypto;

    // RPC
    use self::edge2_gateway_server::Edge2Gateway;
    use std::sync::{Arc, Mutex};
    use tonic::{Request, Response, Status};

    // Include the generated proto file
    tonic::include_proto!("edge2gateway");

    pub struct Edge2GatewayServerStruct {
        e2l_crypto: Arc<Mutex<E2LCrypto>>,
    }

    impl Edge2GatewayServerStruct {
        pub fn new(e2l_crypto: Arc<Mutex<E2LCrypto>>) -> Self {
            Self {
                e2l_crypto: e2l_crypto,
            }
        }
    }

    #[tonic::async_trait]
    impl Edge2Gateway for Edge2GatewayServerStruct {
        async fn handle_ed_pub_info(
            &self,
            request: Request<EdPubInfo>,
        ) -> Result<Response<GwInfo>, Status> {
            let inner_request = request.into_inner();
            let dev_eui = inner_request.dev_eui;
            let dev_addr = inner_request.dev_addr;
            let g_as_ed_compressed = inner_request.g_as_ed;
            let dev_public_key_compressed = inner_request.dev_public_key;
            let e2l_crypto = self.e2l_crypto.lock().expect("Could not lock");
            let g_gw_ed_compressed = e2l_crypto.handle_ed_pub_info(
                dev_eui,
                dev_addr,
                g_as_ed_compressed,
                dev_public_key_compressed,
            );
            std::mem::drop(e2l_crypto);
            // Check if the result is empty
            let reply: GwInfo;
            if g_gw_ed_compressed.is_empty() {
                reply = GwInfo {
                    status_code: -1,
                    g_gw_ed: g_gw_ed_compressed,
                };
            } else {
                reply = GwInfo {
                    status_code: 0,
                    g_gw_ed: g_gw_ed_compressed,
                };
            }
            Ok(Response::new(reply))
        }

        async fn update_aggregation_params(
            &self,
            request: Request<AggregationParams>,
        ) -> Result<Response<GwResponse>, Status> {
            let inner_request = request.into_inner();
            let _aggregation_function: u32 = inner_request.aggregation_function;
            let _window_size: u32 = inner_request.window_size;
            let response = GwResponse {
                status_code: 0,
                message: "Parameters Updated!".to_string(),
            };
            Ok(Response::new(response))
        }

        async fn remove_e2device(
            &self,
            request: Request<E2lDeviceInfo>,
        ) -> Result<Response<E2lData>, Status> {
            let inner_request = request.into_inner();
            let dev_eui = inner_request.dev_eui;
            let _dev_addr = inner_request.dev_addr;
            let e2l_crypto = self.e2l_crypto.lock().expect("Could not lock");
            let response = e2l_crypto.remove_e2device(dev_eui);
            std::mem::drop(e2l_crypto);
            Ok(Response::new(response))
        }

        async fn add_devices(
            &self,
            request: Request<E2lDevicesInfoComplete>,
        ) -> Result<Response<GwResponse>, Status> {
            let inner_request = request.into_inner();
            let device_list = inner_request.device_list;
            let e2l_crypto = self.e2l_crypto.lock().expect("Could not lock");
            let response = e2l_crypto.add_devices(device_list);
            std::mem::drop(e2l_crypto);
            Ok(Response::new(response))
        }

        async fn set_active(
            &self,
            request: Request<ActiveFlag>,
        ) -> Result<Response<GwResponse>, Status> {
            let inner_request = request.into_inner();
            let is_active = inner_request.is_active;
            let e2l_crypto = self.e2l_crypto.lock().expect("Could not lock");
            e2l_crypto.set_active(is_active);
            std::mem::drop(e2l_crypto);
            let response = GwResponse {
                status_code: 0,
                message: "Parameters Updated!".to_string(),
            };
            Ok(Response::new(response))
        }
    }
}
