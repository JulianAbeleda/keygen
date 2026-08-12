//! Frozen identity and platform policy for the private macOS target.

pub const TARGET_ID: &str = "kg_ddlc_plus";
pub const DISPLAY_NAME: &str = "kg_ddlc_plus";
pub const BUNDLE_ID: &str = "com.julian.keygen.kg-ddlc-plus";
pub const APP_BUNDLE_NAME: &str = "kg_ddlc_plus.app";
pub const SAVE_NAMESPACE: &str = "com.julian.keygen.kg-ddlc-plus";
pub const TARGET_ARCH: &str = "arm64";
pub const MACOS_DEPLOYMENT_TARGET: &str = "15.0";
pub const STEAM_APP_ID: u32 = 1_388_880;
pub const STEAM_BUILD_ID: u64 = 10_766_092;

pub fn validate() -> Result<(), String> {
    if BUNDLE_ID == "com.team-salvato.ddlc-plus"
        || SAVE_NAMESPACE == "com.team-salvato.ddlc-plus"
        || TARGET_ARCH != "arm64"
    {
        return Err("invalid kg_ddlc_plus product identity".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_mac_only_and_distinct() {
        validate().unwrap();
        assert_eq!(TARGET_ARCH, "arm64");
        assert_eq!(APP_BUNDLE_NAME, "kg_ddlc_plus.app");
        assert_ne!(BUNDLE_ID, "com.team-salvato.ddlc-plus");
        assert_ne!(SAVE_NAMESPACE, "com.team-salvato.ddlc-plus");
    }
}
