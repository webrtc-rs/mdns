use std::time::Duration;

/// Config is used to configure a mDNS client or server.
#[derive(Debug, Clone)]
pub struct Config {
    /// query_interval controls how often we sends Queries until we
    /// get a response for the requested name
    pub query_interval: Duration,

    /// local_names are the names that we will generate answers for
    /// when we get questions
    pub local_names: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            query_interval: Duration::from_secs(2),
            local_names: vec![],
        }
    }
}
