use crate::provider::mock::MOCK_PROVIDER;
use crate::provider::MailProvider;

#[derive(Clone, Copy)]
pub struct ProviderRegistry {
    default_provider: &'static dyn MailProvider,
}

impl ProviderRegistry {
    pub fn new(default_provider: &'static dyn MailProvider) -> Self {
        Self { default_provider }
    }

    pub fn default_provider(&self) -> &'static dyn MailProvider {
        self.default_provider
    }
}

pub fn default_provider_registry() -> ProviderRegistry {
    ProviderRegistry::new(&MOCK_PROVIDER)
}
