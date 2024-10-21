

pub struct DeprovisionController {
    pub dapr_host: String,
    pub dapr_port: u16,
    pub state_store: String,
}

impl DeprovisionController {
    pub fn new(dapr_host: &str, dapr_port: u16, state_store: &str) -> Self {
        DeprovisionController {
            dapr_host: dapr_host.to_string(),
            dapr_port,
            state_store: state_store.to_string(),
        }
    }

    pub fn deprovision() {
        // Deprovision the router by deleting the subscription info from the state store

        
    }
}