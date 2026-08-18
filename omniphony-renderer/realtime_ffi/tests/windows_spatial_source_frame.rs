// Compile the production Windows spatial ingress + source-scene lowering inside
// the realtime contract test job. This keeps the lossless object path executable
// without requiring a Windows provider to exist on the CI host.
#[path = "../../windows_host/src/spatial_ingress.rs"]
mod spatial_ingress;
#[path = "../../windows_host/src/spatial_source_frame.rs"]
mod spatial_source_frame;
