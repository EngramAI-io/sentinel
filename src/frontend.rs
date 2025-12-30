use rust_embed::{RustEmbed, Embed};

#[derive(RustEmbed)]
#[folder = "frontend/my-react-flow-app/dist"]
pub struct FrontendAssets;