fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Only compile proto files if protoc is available
    // This allows building without gRPC support
    #[cfg(feature = "grpc-codegen")]
    {
        let proto_dir = "src/backend/proto";
        std::fs::create_dir_all(proto_dir)?;
        
        tonic_build::configure()
            .build_server(false)
            .build_client(true)
            .out_dir(proto_dir)
            .compile(&["proto/backend.proto"], &["proto/"])?;
    }
    
    Ok(())
}

