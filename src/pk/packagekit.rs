//! # DBus interface proxies for: `org.freedesktop.PackageKit`, `org.freedesktop.PackageKit.Offline`
//!
//! This code was generated by `zbus-xmlgen` `2.0.1` from DBus introspection data.
//! Source: `Interface '/org/freedesktop/PackageKit' from service 'org.freedesktop.PackageKit' on system bus`.
//!
//! You may prefer to adapt it, instead of using it verbatim.
//!
//! More information can be found in the
//! [Writing a client proxy](https://dbus.pages.freedesktop.org/zbus/client.html)
//! section of the zbus documentation.
//!
//! This DBus object implements
//! [standard DBus interfaces](https://dbus.freedesktop.org/doc/dbus-specification.html),
//! (`org.freedesktop.DBus.*`) for which the following zbus proxies can be used:
//!
//! * [`zbus::fdo::PropertiesProxy`]
//! * [`zbus::fdo::IntrospectableProxy`]
//! * [`zbus::fdo::PeerProxy`]
//!
//! …consequently `zbus-xmlgen` did not generate code for the above interfaces.

use zbus::proxy;

use crate::pk::TransactionProxy;

#[proxy(
    interface = "org.freedesktop.PackageKit",
    default_service = "org.freedesktop.PackageKit",
    default_path = "/org/freedesktop/PackageKit",
    assume_defaults = true
)]
pub trait PackageKit {
    /// CanAuthorize method
    fn can_authorize(&self, action_id: &str) -> zbus::Result<u32>;

    /// CreateTransaction method
    #[zbus(object = "Transaction")]
    fn create_transaction(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    /// GetDaemonState method
    fn get_daemon_state(&self) -> zbus::Result<String>;

    /// GetPackageHistory method
    fn get_package_history(
        &self,
        names: &[&str],
        count: u32,
    ) -> zbus::Result<
        std::collections::HashMap<
            String,
            Vec<std::collections::HashMap<String, zbus::zvariant::OwnedValue>>,
        >,
    >;

    /// GetTimeSinceAction method
    fn get_time_since_action(&self, role: u32) -> zbus::Result<u32>;

    /// GetTransactionList method
    fn get_transaction_list(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;

    /// SetProxy method
    fn set_proxy(
        &self,
        proxy_http: &str,
        proxy_https: &str,
        proxy_ftp: &str,
        proxy_socks: &str,
        no_proxy: &str,
        pac: &str,
    ) -> zbus::Result<()>;

    /// StateHasChanged method
    fn state_has_changed(&self, reason: &str) -> zbus::Result<()>;

    /// SuggestDaemonQuit method
    fn suggest_daemon_quit(&self) -> zbus::Result<()>;

    /// RepoListChanged signal
    #[zbus(signal)]
    fn repo_list_changed(&self) -> zbus::Result<()>;

    /// RestartSchedule signal
    #[zbus(signal)]
    fn restart_schedule(&self) -> zbus::Result<()>;

    /// TransactionListChanged signal
    #[zbus(signal)]
    fn transaction_list_changed(&self, transactions: Vec<&str>) -> zbus::Result<()>;

    /// UpdatesChanged signal
    #[zbus(signal)]
    fn updates_changed(&self) -> zbus::Result<()>;

    /// BackendAuthor property
    #[zbus(property)]
    fn backend_author(&self) -> zbus::Result<String>;

    /// BackendDescription property
    #[zbus(property)]
    fn backend_description(&self) -> zbus::Result<String>;

    /// BackendName property
    #[zbus(property)]
    fn backend_name(&self) -> zbus::Result<String>;

    /// DistroId property
    #[zbus(property)]
    fn distro_id(&self) -> zbus::Result<String>;

    /// Filters property
    #[zbus(property)]
    fn filters(&self) -> zbus::Result<u64>;

    /// Groups property
    #[zbus(property)]
    fn groups(&self) -> zbus::Result<u64>;

    /// Locked property
    #[zbus(property)]
    fn locked(&self) -> zbus::Result<bool>;

    /// MimeTypes property
    #[zbus(property)]
    fn mime_types(&self) -> zbus::Result<Vec<String>>;

    /// NetworkState property
    #[zbus(property)]
    fn network_state(&self) -> zbus::Result<u32>;

    /// Roles property
    #[zbus(property)]
    fn roles(&self) -> zbus::Result<u64>;

    /// VersionMajor property
    #[zbus(property)]
    fn version_major(&self) -> zbus::Result<u32>;

    /// VersionMicro property
    #[zbus(property)]
    fn version_micro(&self) -> zbus::Result<u32>;

    /// VersionMinor property
    #[zbus(property)]
    fn version_minor(&self) -> zbus::Result<u32>;
}
