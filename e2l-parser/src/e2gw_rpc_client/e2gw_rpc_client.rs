pub(crate) mod e2gw_rpc_client {
    use tonic::transport::Channel;

    use crate::e2gw_rpc_client::e2gw_rpc_client::e2gw_rpc_client::edge2_application_server_client::Edge2ApplicationServerClient;

    // Include the generated proto file
    tonic::include_proto!("edge2applicationserver");

    pub async fn init_rpc_client(
        host: String,
        port: String,
    ) -> Result<Edge2ApplicationServerClient<Channel>, tonic::transport::Error> {
        // Init Edge2LoRa RPC CLient
        println!("Initializing Edge2LoRa RPC Client");
        // Edge2ApplicationServer::
        let rpc_client = Edge2ApplicationServerClient::connect(format!("http://{}:{}", host, port));
        println!("Initialized Edge2LoRa RPC Client");
        // return rpc_client
        rpc_client.await
    }
}
