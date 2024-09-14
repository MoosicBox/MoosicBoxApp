#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]

use moosicbox_async_service::{tokio::sync::RwLock, Arc, CancellationToken, JoinHandle};
use strum_macros::AsRefStr;
use tauri::RunEvent;

#[derive(Debug, AsRefStr)]
pub enum Command {
    RunEvent { event: Arc<RunEvent> },
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

pub mod service {
    moosicbox_async_service::async_service!(super::Command, super::Context);
}

#[moosicbox_async_service::async_trait]
impl service::Processor for service::Service {
    type Error = service::Error;

    async fn on_start(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn on_shutdown(_ctx: Arc<RwLock<Context>>) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn process_command(
        ctx: Arc<RwLock<Context>>,
        command: Command,
    ) -> Result<(), Self::Error> {
        log::debug!("process_command command={command}");
        match command {
            Command::RunEvent { event } => {
                log::debug!("Received RunEvent command");
                if let Err(e) = ctx.read().await.handle_event(event) {
                    log::error!("Failed to handle event: {e:?}");
                }
            }
        }
        Ok(())
    }
}

pub struct Context {
    server_handle: JoinHandle<Result<(), std::io::Error>>,
    token: CancellationToken,
}

impl Default for Context {
    fn default() -> Self {
        let tauri::async_runtime::RuntimeHandle::Tokio(handle) = tauri::async_runtime::handle();

        let server_handle = moosicbox_task::spawn_on(
            "moosicbox_app_bundled server",
            &handle,
            moosicbox_server::run("0.0.0.0", 8016, None),
        );

        Self {
            server_handle,
            token: CancellationToken::new(),
        }
    }
}

impl Context {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle_event(&self, event: Arc<RunEvent>) -> Result<(), std::io::Error> {
        match *event {
            tauri::RunEvent::Exit { .. } => {}
            tauri::RunEvent::ExitRequested { .. } => {
                self.shutdown()?;
            }
            tauri::RunEvent::WindowEvent { .. } => {}
            tauri::RunEvent::Ready => {}
            tauri::RunEvent::Resumed => {}
            tauri::RunEvent::MainEventsCleared => {}
            _ => {}
        }
        Ok(())
    }

    pub fn shutdown(&self) -> Result<(), std::io::Error> {
        self.server_handle.abort();
        self.token.cancel();
        Ok(())
    }
}
