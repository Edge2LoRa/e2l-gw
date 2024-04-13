pub(crate) mod e2gw_rpc_server {
    use crate::e2l_crypto::e2l_crypto::E2L_CRYPTO;

    // RPC
    use self::edge2_gateway_server::Edge2Gateway;
    use tonic::{Request, Response, Status};

    // Include the generated proto file
    tonic::include_proto!("edge2gateway");

    pub struct MyEdge2GatewayServer {}

    #[tonic::async_trait]
    impl Edge2Gateway for MyEdge2GatewayServer {
        async fn handle_ed_pub_info(
            &self,
            request: Request<EdPubInfo>,
        ) -> Result<Response<GwInfo>, Status> {
            let inner_request = request.into_inner();
            let dev_eui = inner_request.dev_eui;
            let dev_addr = inner_request.dev_addr;
            let g_as_ed_compressed = inner_request.g_as_ed;
            let dev_public_key_compressed = inner_request.dev_public_key;
            unsafe {
                let g_gw_ed_compressed = E2L_CRYPTO.handle_ed_pub_info(
                    dev_eui,
                    dev_addr,
                    g_as_ed_compressed,
                    dev_public_key_compressed,
                );
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
        }

        async fn update_aggregation_params(
            &self,
            request: Request<AggregationParams>,
        ) -> Result<Response<GwResponse>, Status> {
            let inner_request = request.into_inner();
            let aggregation_function: u32 = inner_request.aggregation_function;
            let window_size: u32 = inner_request.window_size;
            unsafe {
                let _ret = E2L_CRYPTO
                    .update_aggregation_params(aggregation_function as u8, window_size as usize);
            }
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
            unsafe {
                let response: E2lData = E2L_CRYPTO.remove_e2device(dev_eui);
                Ok(Response::new(response))
            }
        }

        async fn add_devices(
            &self,
            request: Request<E2lDevicesInfoComplete>,
        ) -> Result<Response<GwResponse>, Status> {
            let inner_request = request.into_inner();
            let device_list = inner_request.device_list;
            unsafe {
                let response = E2L_CRYPTO.add_devices(device_list);
                Ok(Response::new(response))
            }
        }

        async fn set_active(
            &self,
            request: Request<ActiveFlag>,
        ) -> Result<Response<GwResponse>, Status> {
            let inner_request = request.into_inner();
            let is_active = inner_request.is_active;
            unsafe {
                E2L_CRYPTO.set_active(is_active);
            }
            let response = GwResponse {
                status_code: 0,
                message: "Parameters Updated!".to_string(),
            };
            Ok(Response::new(response))
        }
    }
}
