const API_ADDRESS: &str = "https://api.enso.finance";

pub enum Version {
    V1,
}

impl ToString for Version {
    fn to_string(&self) -> String {
        match self {
            Version::V1 => "v1".to_string(),
        }
    }
}

pub struct Enso {
    api_address: String,
    pub(crate) api_key: String,
    version: String,
}

impl Enso {
    pub fn new<T: ToString>(api_key: T, version: Version) -> Enso {
        Enso {
            api_address: API_ADDRESS.to_string(),
            api_key: api_key.to_string(),
            version: version.to_string(),
        }
    }

    pub fn get_api_url(&self) -> String {
        format!("{}/api/{}", self.api_address, self.version)
    }
}
