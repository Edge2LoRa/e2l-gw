fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("protos/edge2applicationserver.proto")?;
    tonic_build::compile_protos("protos/edge2gateway.proto")?;
    Ok(())
}
