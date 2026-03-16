pub mod oauth {
    pub const PROXY_TOKEN_ENV: &str = "MAILYOU_OAUTH_PROXY_TOKEN";
    pub const PROXY_AUTH_TOKEN: &str = "Lu6WVgtL31TkaXWVeVBIaB8T8CsU3jMfXoxbpomAuas5hF5wpOx5IWfdUiokkc5G";
    pub const DEFAULT_PROXY_BASE_URL: &str = "https://oauth2-proxy.iscccc.cc";
    pub const PROXY_URL_ENV: &str = "MAILYOU_OAUTH_PROXY_URL";

    pub struct DirectProviderConfig {
        pub id: &'static str,
        pub token_url: &'static str,
        pub client_id_env: &'static str,
        pub client_secret_env: &'static str,
    }

    pub const DIRECT_PROVIDERS: &[DirectProviderConfig] = &[
        DirectProviderConfig {
            id: "gmail",
            token_url: "https://oauth2.googleapis.com/token",
            client_id_env: "GMAIL_CLIENT_ID",
            client_secret_env: "GMAIL_CLIENT_SECRET",
        },
        DirectProviderConfig {
            id: "outlook",
            token_url: "https://login.microsoftonline.com/common/oauth2/v2.0/token",
            client_id_env: "OUTLOOK_CLIENT_ID",
            client_secret_env: "OUTLOOK_CLIENT_SECRET",
        },
    ];
}
