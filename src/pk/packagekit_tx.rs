//! # DBus interface proxy for: `org.freedesktop.PackageKit.Transaction`
//!
//! This code was generated by `zbus-xmlgen` `2.0.1` from DBus introspection data.
//! Source: `org.freedesktop.PackageKit.Transaction.xml`.
//!
//! You may prefer to adapt it, instead of using it verbatim.
//!
//! More information can be found in the
//! [Writing a client proxy](https://dbus.pages.freedesktop.org/zbus/client.html)
//! section of the zbus documentation.
//!

use zbus::proxy;

#[proxy(
    interface = "org.freedesktop.PackageKit.Transaction",
    default_service = "org.freedesktop.PackageKit"
)]
pub trait Transaction {
    /// AcceptEula method
    fn accept_eula(&self, eula_id: &str) -> zbus::Result<()>;

    /// Cancel method
    fn cancel(&self) -> zbus::Result<()>;

    /// DependsOn method
    fn depends_on(&self, filter: u64, package_ids: &[&str], recursive: bool) -> zbus::Result<()>;

    /// DownloadPackages method
    fn download_packages(&self, store_in_cache: bool, package_ids: &[&str]) -> zbus::Result<()>;

    /// GetCategories method
    fn get_categories(&self) -> zbus::Result<()>;

    /// GetDetails method
    fn get_details(&self, package_ids: &[&str]) -> zbus::Result<()>;

    /// GetDetailsLocal method
    fn get_details_local(&self, files: &[&str]) -> zbus::Result<()>;

    /// GetDistroUpgrades method
    fn get_distro_upgrades(&self) -> zbus::Result<()>;

    /// GetFiles method
    fn get_files(&self, package_ids: &[&str]) -> zbus::Result<()>;

    /// GetFilesLocal method
    fn get_files_local(&self, files: &[&str]) -> zbus::Result<()>;

    /// GetOldTransactions method
    fn get_old_transactions(&self, number: u32) -> zbus::Result<()>;

    /// GetPackages method
    fn get_packages(&self, filter: u64) -> zbus::Result<()>;

    /// GetRepoList method
    fn get_repo_list(&self, filter: u64) -> zbus::Result<()>;

    /// GetUpdateDetail method
    fn get_update_detail(&self, package_ids: &[&str]) -> zbus::Result<()>;

    /// GetUpdates method
    fn get_updates(&self, filter: u64) -> zbus::Result<()>;

    /// InstallFiles method
    fn install_files(&self, transaction_flags: u64, full_paths: &[&str]) -> zbus::Result<()>;

    /// InstallPackages method
    fn install_packages(&self, transaction_flags: u64, package_ids: &[&str]) -> zbus::Result<()>;

    /// InstallSignature method
    fn install_signature(&self, sig_type: u32, key_id: &str, package_id: &str) -> zbus::Result<()>;

    /// RefreshCache method
    fn refresh_cache(&self, force: bool) -> zbus::Result<()>;

    /// RemovePackages method
    fn remove_packages(
        &self,
        transaction_flags: u64,
        package_ids: &[&str],
        allow_deps: bool,
        autoremove: bool,
    ) -> zbus::Result<()>;

    /// RepairSystem method
    fn repair_system(&self, transaction_flags: u64) -> zbus::Result<()>;

    /// RepoEnable method
    fn repo_enable(&self, repo_id: &str, enabled: bool) -> zbus::Result<()>;

    /// RepoRemove method
    fn repo_remove(
        &self,
        transaction_flags: u64,
        repo_id: &str,
        autoremove: bool,
    ) -> zbus::Result<()>;

    /// RepoSetData method
    fn repo_set_data(&self, repo_id: &str, parameter: &str, value: &str) -> zbus::Result<()>;

    /// RequiredBy method
    fn required_by(&self, filter: u64, package_ids: &[&str], recursive: bool) -> zbus::Result<()>;

    /// Resolve method
    fn resolve(&self, filter: u64, packages: &[&str]) -> zbus::Result<()>;

    /// SearchDetails method
    fn search_details(&self, filter: u64, values: &[&str]) -> zbus::Result<()>;

    /// SearchFiles method
    fn search_files(&self, filter: u64, values: &[&str]) -> zbus::Result<()>;

    /// SearchGroups method
    fn search_groups(&self, filter: u64, values: &[&str]) -> zbus::Result<()>;

    /// SearchNames method
    fn search_names(&self, filter: u64, values: &[&str]) -> zbus::Result<()>;

    /// SetHints method
    fn set_hints(&self, hints: &[&str]) -> zbus::Result<()>;

    /// UpdatePackages method
    fn update_packages(&self, transaction_flags: u64, package_ids: &[&str]) -> zbus::Result<()>;

    /// UpgradeSystem method
    fn upgrade_system(
        &self,
        transaction_flags: u64,
        distro_id: &str,
        upgrade_kind: u32,
    ) -> zbus::Result<()>;

    /// WhatProvides method
    fn what_provides(&self, filter: u64, values: &[&str]) -> zbus::Result<()>;

    /// Category signal
    #[zbus(signal)]
    fn category(
        &self,
        parent_id: &str,
        cat_id: &str,
        name: &str,
        summary: &str,
        icon: &str,
    ) -> zbus::Result<()>;

    /// Destroy signal
    #[zbus(signal)]
    fn destroy(&self) -> zbus::Result<()>;

    /// Details signal
    #[zbus(signal)]
    fn details(
        &self,
        data: std::collections::HashMap<&str, zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;

    /// DistroUpgrade signal
    #[zbus(signal)]
    fn distro_upgrade(&self, type_: u32, name: &str, summary: &str) -> zbus::Result<()>;

    /// ErrorCode signal
    #[zbus(signal)]
    fn error_code(&self, code: u32, details: &str) -> zbus::Result<()>;

    /// EulaRequired signal
    #[zbus(signal)]
    fn eula_required(
        &self,
        eula_id: &str,
        package_id: &str,
        vendor_name: &str,
        license_agreement: &str,
    ) -> zbus::Result<()>;

    /// Files signal
    #[zbus(signal)]
    fn files(&self, package_id: &str, file_list: Vec<&str>) -> zbus::Result<()>;

    /// Finished signal
    #[zbus(signal)]
    fn finished(&self, exit: u32, runtime: u32) -> zbus::Result<()>;

    /// ItemProgress signal
    #[zbus(signal)]
    fn item_progress(&self, id: &str, status: u32, percentage: u32) -> zbus::Result<()>;

    /// MediaChangeRequired signal
    #[zbus(signal)]
    fn media_change_required(
        &self,
        media_type: u32,
        media_id: &str,
        media_text: &str,
    ) -> zbus::Result<()>;

    /// Package signal
    #[zbus(signal)]
    fn package(&self, info: u32, package_id: &str, summary: &str) -> zbus::Result<()>;

    /// RepoDetail signal
    #[zbus(signal)]
    fn repo_detail(&self, repo_id: &str, description: &str, enabled: bool) -> zbus::Result<()>;

    /// RepoSignatureRequired signal
    #[zbus(signal)]
    fn repo_signature_required(
        &self,
        package_id: &str,
        repository_name: &str,
        key_url: &str,
        key_userid: &str,
        key_id: &str,
        key_fingerprint: &str,
        key_timestamp: &str,
        type_: u32,
    ) -> zbus::Result<()>;

    /// RequireRestart signal
    #[zbus(signal)]
    fn require_restart(&self, type_: u32, package_id: &str) -> zbus::Result<()>;

    /// Transaction signal
    #[zbus(signal)]
    fn transaction(
        &self,
        object_path: zbus::zvariant::ObjectPath<'_>,
        timespec: &str,
        succeeded: bool,
        role: u32,
        duration: u32,
        data: &str,
        uid: u32,
        cmdline: &str,
    ) -> zbus::Result<()>;

    /// UpdateDetail signal
    #[zbus(signal)]
    fn update_detail(
        &self,
        package_id: &str,
        updates: Vec<&str>,
        obsoletes: Vec<&str>,
        vendor_urls: Vec<&str>,
        bugzilla_urls: Vec<&str>,
        cve_urls: Vec<&str>,
        restart: u32,
        update_text: &str,
        changelog: &str,
        state: u32,
        issued: &str,
        updated: &str,
    ) -> zbus::Result<()>;

    /// AllowCancel property
    #[zbus(property)]
    fn allow_cancel(&self) -> zbus::Result<bool>;

    /// CallerActive property
    #[zbus(property)]
    fn caller_active(&self) -> zbus::Result<bool>;

    /// DownloadSizeRemaining property
    #[zbus(property)]
    fn download_size_remaining(&self) -> zbus::Result<u64>;

    /// ElapsedTime property
    #[zbus(property)]
    fn elapsed_time(&self) -> zbus::Result<u32>;

    /// LastPackage property
    #[zbus(property)]
    fn last_package(&self) -> zbus::Result<String>;

    /// Percentage property
    #[zbus(property)]
    fn percentage(&self) -> zbus::Result<u32>;

    /// RemainingTime property
    #[zbus(property)]
    fn remaining_time(&self) -> zbus::Result<u32>;

    /// Role property
    #[zbus(property)]
    fn role(&self) -> zbus::Result<u32>;

    /// Speed property
    #[zbus(property)]
    fn speed(&self) -> zbus::Result<u32>;

    /// Status property
    #[zbus(property)]
    fn status(&self) -> zbus::Result<u32>;

    /// TransactionFlags property
    #[zbus(property)]
    fn transaction_flags(&self) -> zbus::Result<u64>;

    /// Uid property
    #[zbus(property)]
    fn uid(&self) -> zbus::Result<u32>;
}
