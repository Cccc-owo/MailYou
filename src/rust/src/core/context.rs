use crate::provider::registry::{default_provider_registry, ProviderRegistry};
use crate::provider::MailProvider;

pub struct AppContext {
    registry: ProviderRegistry,
}

impl AppContext {
    pub fn new(registry: ProviderRegistry) -> Self {
        Self { registry }
    }

    pub fn provider(&self) -> &'static dyn MailProvider {
        self.registry.default_provider()
    }
}

pub fn app_context() -> AppContext {
    AppContext::new(default_provider_registry())
}
