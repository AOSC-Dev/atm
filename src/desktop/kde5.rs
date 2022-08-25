#![allow(non_snake_case)] // for generated code

use zbus::dbus_proxy;

// generated code -->

#[dbus_proxy(
    interface = "org.kde.JobViewServerV2",
    default_service = "org.kde.kuiserver",
    default_path = "/JobViewServer"
)]
trait JobViewServerV2 {
    /// requestView method
    #[dbus_proxy(name = "requestView")]
    fn request_view(
        &self,
        desktopEntry: &str,
        capabilities: i32,
        hints: std::collections::HashMap<&str, zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
}

#[dbus_proxy(interface = "org.kde.JobViewV2", default_service = "org.kde.kuiserver")]
trait JobViewV2 {
    /// clearDescriptionField method
    fn clear_description_field(&self, number: u32) -> zbus::Result<()>;

    /// setDescriptionField method
    #[dbus_proxy(name = "setDescriptionField")]
    fn set_description_field(&self, number: u32, name: &str, value: &str) -> zbus::Result<bool>;

    /// setDestUrl method
    // fn set_dest_url(&self, destUrl: &zbus::zvariant::Value<'_>) -> zbus::Result<()>;

    /// setError method
    // fn set_error(&self, errorCode: u32) -> zbus::Result<()>;

    /// setInfoMessage method
    #[dbus_proxy(name = "setInfoMessage")]
    fn set_info_message(&self, message: &str) -> zbus::Result<()>;

    /// setPercent method
    #[dbus_proxy(name = "setPercent")]
    fn set_percent(&self, percent: u32) -> zbus::Result<()>;

    /// setProcessedAmount method
    fn set_processed_amount(&self, amount: u64, unit: &str) -> zbus::Result<()>;

    /// setSpeed method
    // fn set_speed(&self, bytesPerSecond: u64) -> zbus::Result<()>;

    /// setSuspended method
    fn set_suspended(&self, suspended: bool) -> zbus::Result<()>;

    /// setTotalAmount method
    fn set_total_amount(&self, amount: u64, unit: &str) -> zbus::Result<()>;

    /// terminate method
    #[dbus_proxy(name = "terminate")]
    fn terminate(&self, errorMessage: &str) -> zbus::Result<()>;

    /// cancelRequested signal
    #[dbus_proxy(signal)]
    fn cancel_requested(&self) -> zbus::Result<()>;

    /// resumeRequested signal
    #[dbus_proxy(signal)]
    fn resume_requested(&self) -> zbus::Result<()>;

    /// suspendRequested signal
    #[dbus_proxy(signal)]
    fn suspend_requested(&self) -> zbus::Result<()>;
}

// <-- end of generated code

// implementation for progress tracker
use super::ProgressTracker;

use anyhow::Result;

pub struct KF5Tracker<'a> {
    async_runner: tokio::runtime::Runtime,
    pub dbus_connection: zbus::Connection,
    job_proxy: JobViewV2Proxy<'a>,
}

impl ProgressTracker for KF5Tracker<'_> {
    fn set_percent(&mut self, percent: u32) {
        self.async_runner
            .block_on(self.job_proxy.set_percent(percent))
            .ok();
    }

    fn set_general_description(&mut self, description: &str) {
        self.async_runner
            .block_on(self.job_proxy.set_info_message(description))
            .ok();
    }

    fn set_message(&mut self, label: &str, message: &str) {
        self.async_runner
            .block_on(self.job_proxy.set_description_field(0, label, message))
            .ok();
    }

    fn terminate(&mut self, message: &str) {
        self.async_runner
            .block_on(self.job_proxy.terminate(message))
            .ok();
    }
}

fn create_async_runner() -> Result<tokio::runtime::Runtime, std::io::Error> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
}

impl KF5Tracker<'_> {
    pub fn new(app_id: &str) -> Result<Self> {
        let runner = create_async_runner()?;
        let (connection, proxy) = runner.block_on(async {
            let conn = zbus::Connection::session().await?;
            let server = JobViewServerV2Proxy::new(&conn).await?;
            let path = server
                .request_view(app_id, 0, std::collections::HashMap::new())
                .await?;
            let proxy = JobViewV2Proxy::builder(&conn).path(path)?.build().await?;

            Ok::<_, anyhow::Error>((conn, proxy))
        })?;

        Ok(Self {
            async_runner: runner,
            dbus_connection: connection,
            job_proxy: proxy,
        })
    }
}

impl Drop for KF5Tracker<'_> {
    fn drop(&mut self) {
        self.terminate("")
    }
}
