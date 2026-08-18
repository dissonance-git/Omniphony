// Compile the Windows Spatial Audio ingress contract as an integration test
// before it is wired to a platform host. This keeps the semantic boundary
// executable without pretending that a raw system-wide object interception
// mechanism has already been proven.
#[path = "../src/windows_spatial_contract.rs"]
mod windows_spatial_contract;
