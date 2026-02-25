fn main() {
    #[cfg(feature = "xai-grpc")]
    {
        let proto_root = std::path::Path::new("../xai-proto/proto");
        if proto_root.exists() {
            tonic_prost_build::configure()
                .build_server(false) // client-only: we call xAI, not serve
                .compile_protos(
                    &[
                        "../xai-proto/proto/xai/api/v1/chat.proto",
                        "../xai-proto/proto/xai/api/v1/embed.proto",
                        "../xai-proto/proto/xai/api/v1/models.proto",
                        "../xai-proto/proto/xai/api/v1/tokenize.proto",
                    ],
                    &["../xai-proto/proto"],
                )
                .expect("Failed to compile xAI gRPC protos");
        } else {
            println!(
                "cargo:warning=xai-proto not found at ../xai-proto — \
                 clone https://github.com/xai-org/xai-proto.git alongside crewai-rust"
            );
        }
    }
}
