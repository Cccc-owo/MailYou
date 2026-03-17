use crate::provider::registry::{default_provider_registry, ProviderRegistry};

pub struct AppContext {
    registry: ProviderRegistry,
}

impl AppContext {
    pub fn new(registry: ProviderRegistry) -> Self {
        Self { registry }
    }

    pub fn registry(&self) -> &ProviderRegistry {
        &self.registry
    }
}

pub fn app_context() -> AppContext {
    AppContext::new(default_provider_registry())
}
