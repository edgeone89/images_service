fn main() {
    tonic_build::compile_protos("proto/image.proto").unwrap();
}
