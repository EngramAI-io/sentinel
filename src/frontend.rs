use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "frontend/my-react-flow-app/dist"]
pub struct FrontendAssets;