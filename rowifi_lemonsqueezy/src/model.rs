use serde::Deserialize;

#[derive(Deserialize)]
pub struct LicenseActivation {
    pub activated: bool,
    pub error: Option<String>,
    pub instance: Option<LicenseInstance>,
    pub meta: LicenseMeta,
}

#[derive(Deserialize)]
pub struct LicenseDeactivation {
    pub deactivated: bool,
}

#[derive(Deserialize)]
pub struct LicenseInstance {
    pub id: String,
    pub name: String,
}

#[derive(Deserialize)]
pub struct LicenseMeta {
    pub product_id: u64,
}

#[derive(Deserialize)]
pub struct LicenseValidation {
    pub valid: bool,
}
