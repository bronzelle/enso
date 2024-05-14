use std::fmt::Display;

const API_ADDRESS: &str = "https://api.enso.finance";

pub enum Version {
    V1,
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Version::V1 => "v1".to_string(),
            }
        )
    }
}

pub struct Enso {
    api_address: String,
    pub(crate) api_key: String,
    version: String,
}

impl Enso {
    /// Creates a new `Enso` instance.
    ///
    /// # Arguments
    ///
    /// * `api_key` - The API key for accessing Enso.
    /// * `version` - The API version to use.
    ///
    /// # Example
    ///
    /// ```
    /// let enso = Enso::new("your_api_key", Version::V1);
    /// ```
    pub fn new<T: ToString>(api_key: T, version: Version) -> Enso {
        Enso {
            api_address: API_ADDRESS.to_string(),
            api_key: api_key.to_string(),
            version: version.to_string(),
        }
    }

    pub(crate) fn get_api_url(&self) -> String {
        format!("{}/api/{}", self.api_address, self.version)
    }
}
