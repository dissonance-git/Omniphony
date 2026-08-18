// Compile the production Windows-host ingress adapter inside the already-gated
// realtime contract test job. This keeps the raw-object seam executable before
// the experimental system provider is allowed into installer packaging.
#[path = "../../windows_host/src/spatial_ingress.rs"]
mod spatial_ingress;
