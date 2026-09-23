//! The provider that ships today: a church's own keys, from the OS credential
//! store, with a developer's `.env` behind it.
//!
//! This is BYOK from Phase 2 of `docs/SermonAI_API_Key_Strategy_Prompt.md`,
//! brought forward ahead of the gateway so an installed release build can be
//! made to work at all. Before it, a release build had no credentials by
//! construction and reported `NotActivated` — correct, but it also meant the
//! app could not transcribe on the machine it was installed on, and M1's
//! latency and memory figures had to come from an optimised build.
//!
//! ## Precedence: keychain, then `.env`
//!
//! A key pasted into Settings wins over one in `.env`, because it is the more
//! deliberate act — somebody sat in front of this machine and chose it. The
//! fallback only exists in debug builds, and the settings panel names which
//! source is in use so a developer is never left wondering why their `.env`
//! appears to be ignored.
//!
//! ## What this provider still cannot do
//!
//! Issue managed access. YouVersion and Tyndale are managed-only, because the
//! licence is SermonAI's rather than the church's, so on this provider they
//! report `NotActivated` and there is nothing the operator can paste to change
//! that. The gateway (Phase 3) is what fills them in.

use super::keychain::KeychainStore;
use super::{Access, CredentialStatus, KeySource, Provider, Secret, Service};
use crate::error::{Error, Result};

pub struct LocalProvider {
    store: KeychainStore,
}

impl LocalProvider {
    pub fn new() -> Self {
        Self {
            store: KeychainStore::new(),
        }
    }

    #[cfg(test)]
    pub fn with_store(store: KeychainStore) -> Self {
        Self { store }
    }

    /// A stored key and where it came from.
    ///
    /// Returns the error from the credential store rather than swallowing it:
    /// "Credential Manager would not open" and "no key is set" need different
    /// things from the operator, and reporting the first as the second sends
    /// them to paste a key they have already pasted.
    fn resolve(&self, service: Service) -> Result<Option<(Secret, KeySource)>> {
        if let Some(secret) = self.store.get(service)? {
            return Ok(Some((secret, KeySource::Keychain)));
        }
        Ok(Self::dev_env_key(service).map(|secret| (secret, KeySource::DevEnv)))
    }

    /// A key from the environment. **Development builds only** — the body is
    /// compiled out of a release build, so a shipped installer has no path from
    /// an environment variable to a vendor call. Setting `DEEPGRAM_API_KEY` on
    /// a church machine must not quietly bill somebody's account.
    fn dev_env_key(service: Service) -> Option<Secret> {
        #[cfg(debug_assertions)]
        {
            let raw = std::env::var(service.dev_env_var()).ok()?;
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return None;
            }
            Some(Secret::new(trimmed))
        }

        #[cfg(not(debug_assertions))]
        {
            let _ = service;
            None
        }
    }
}

impl Default for LocalProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Provider for LocalProvider {
    fn access(&self, service: Service) -> Result<Access> {
        match self.resolve(service)? {
            Some((secret, _)) => Ok(Access::DirectKey(secret)),
            // Names where to go, not what is wrong. An operator who reads
            // "not activated" mid-service needs the next click, and the one
            // for a managed-only service is different from the one for a
            // service they can paste a key into.
            None if service.allows_byok() => Err(Error::Config(format!(
                "No {} key is set. Add one in Settings → Services and keys.",
                service.label()
            ))),
            None => Err(Error::Config(format!(
                "{} is not available yet. It comes with SermonAI activation, which is not built.",
                service.label()
            ))),
        }
    }

    fn status(&self, service: Service) -> CredentialStatus {
        match self.resolve(service) {
            Ok(Some((secret, source))) => CredentialStatus::ByokActive {
                masked: secret.masked(),
                source,
            },
            Ok(None) => CredentialStatus::NotActivated,
            // A store that will not open is not the same as an empty one, and
            // the settings panel must not show "no key" when the truth is
            // "could not look".
            Err(err) => CredentialStatus::StoreUnavailable {
                detail: err.to_string(),
            },
        }
    }

    fn set_byok(&self, service: Service, key: &str) -> Result<()> {
        super::Credentials::check_byok_allowed(service)?;
        self.store.set(service, key)
    }

    fn remove_byok(&self, service: Service) -> Result<()> {
        self.store.remove(service)
    }
}

/// Used when even the local store could not be constructed. Reports that
/// nothing is activated rather than failing startup, so the operator reaches a
/// window that can tell them what is wrong.
pub struct UnconfiguredProvider;

impl Provider for UnconfiguredProvider {
    fn access(&self, service: Service) -> Result<Access> {
        Err(Error::Config(format!(
            "{} is not available. SermonAI could not open this machine's credential store.",
            service.label()
        )))
    }

    fn status(&self, _service: Service) -> CredentialStatus {
        CredentialStatus::NotActivated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> LocalProvider {
        LocalProvider::with_store(KeychainStore::with_namespace(format!(
            "SermonAI-test-local-{name}-{}",
            std::process::id()
        )))
    }

    #[test]
    fn a_pasted_key_is_reported_masked_and_usable() {
        // Every test that asserts on a *missing* key has to hold the env lock
        // and clear the variable. The keychain namespaces are per-test, but the
        // environment is per-process, so without this the precedence test's
        // `DEEPGRAM_API_KEY` answers here and the final assertion sees a key
        // that this test never stored.
        let _lock = env_lock();
        let _guard = EnvGuard::set("DEEPGRAM_API_KEY", None);
        let provider = scratch("pasted");
        provider
            .set_byok(Service::Deepgram, "dg_pasted_wxyz")
            .expect("save");

        assert_eq!(
            provider.status(Service::Deepgram),
            CredentialStatus::ByokActive {
                masked: "••••wxyz".to_string(),
                source: KeySource::Keychain,
            }
        );

        match provider.access(Service::Deepgram).expect("usable") {
            Access::DirectKey(secret) => assert_eq!(secret.expose(), "dg_pasted_wxyz"),
            Access::Gateway { .. } => panic!("this provider issues no gateway access"),
        }

        provider.remove_byok(Service::Deepgram).expect("cleanup");
        assert_eq!(
            provider.status(Service::Deepgram),
            CredentialStatus::NotActivated
        );
    }

    #[test]
    fn a_managed_only_service_refuses_a_pasted_key() {
        // The check belongs to the provider rather than the settings panel, so
        // the rule holds for any caller. A church's YouVersion key would
        // authenticate and then return nothing it is licensed to display,
        // which looks like our bug.
        let provider = scratch("managed-only");

        for service in [Service::YouVersion, Service::TyndaleNlt] {
            let err = provider
                .set_byok(service, "whatever")
                .expect_err("managed-only services take no pasted key");
            assert!(err.to_string().contains("licence"), "{err}");
        }
    }

    #[test]
    fn the_message_for_a_missing_key_names_where_to_set_it() {
        // Under the same lock as the precedence test, and with the variable
        // explicitly cleared: env vars are process-global, so without this the
        // two tests race and this one finds the other's key.
        let _lock = env_lock();
        let _guard = EnvGuard::set("DEEPGRAM_API_KEY", None);
        let provider = scratch("missing");

        // The exact strings, not a substring check. The Live tab decides
        // whether to offer an "Open Services and keys" button by matching this
        // wording — `Error::Config` carries no code — so a reword here would
        // silently drop the button. The frontend pins the same two literals in
        // `ServiceSettings.test.ts`.
        assert_eq!(
            provider
                .access(Service::Deepgram)
                .expect_err("nothing stored")
                .to_string(),
            "No Deepgram key is set. Add one in Settings → Services and keys."
        );

        // Managed-only: sending them to a paste box they cannot use would be
        // worse than saying the feature is not built.
        assert_eq!(
            provider
                .access(Service::TyndaleNlt)
                .expect_err("managed only")
                .to_string(),
            "Tyndale NLT is not available yet. It comes with SermonAI activation, which is not built."
        );
    }

    #[test]
    fn a_stored_key_wins_over_the_environment() {
        // Precedence matters both ways round: a developer with a working .env
        // who pastes a key in Settings must get the pasted one, and the panel
        // has to be able to say which is in use.
        let _lock = env_lock();
        let _guard = EnvGuard::set("DEEPGRAM_API_KEY", Some("dg_from_env_1111"));
        let provider = scratch("precedence");

        assert_eq!(
            provider.status(Service::Deepgram),
            CredentialStatus::ByokActive {
                masked: "••••1111".to_string(),
                source: KeySource::DevEnv,
            },
            "with nothing stored, the environment answers"
        );

        provider
            .set_byok(Service::Deepgram, "dg_from_store_2222")
            .expect("save");

        assert_eq!(
            provider.status(Service::Deepgram),
            CredentialStatus::ByokActive {
                masked: "••••2222".to_string(),
                source: KeySource::Keychain,
            },
            "a pasted key must win over .env"
        );

        // And removing it falls back rather than going dark.
        provider.remove_byok(Service::Deepgram).expect("cleanup");
        assert_eq!(
            provider.status(Service::Deepgram),
            CredentialStatus::ByokActive {
                masked: "••••1111".to_string(),
                source: KeySource::DevEnv,
            }
        );
    }

    // Env vars are process-global; these share one lock rather than racing.
    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    struct EnvGuard {
        var: &'static str,
        previous: Option<String>,
    }

    impl EnvGuard {
        fn set(var: &'static str, value: Option<&str>) -> Self {
            let previous = std::env::var(var).ok();
            match value {
                Some(v) => std::env::set_var(var, v),
                None => std::env::remove_var(var),
            }
            Self { var, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(v) => std::env::set_var(self.var, v),
                None => std::env::remove_var(self.var),
            }
        }
    }
}
