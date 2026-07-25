use nats_backend::BackendCommand;

use super::model::EasyNatsApp;

impl EasyNatsApp {
    pub(crate) fn request_object_upload(&mut self, connection_id: u64, bucket: String) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let Some(path) = rfd::FileDialog::new().pick_file() else {
                return;
            };
            let Ok(data) = std::fs::read(&path) else {
                return;
            };
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| "unnamed".to_string());
            self.backend.send(BackendCommand::UploadObject {
                connection_id,
                bucket,
                name,
                data,
            });
        }

        #[cfg(target_arch = "wasm32")]
        self.backend.send(BackendCommand::UploadObject {
            connection_id,
            bucket,
            name: "demo-upload.json".to_string(),
            data: br#"{"source":"interactive-demo","status":"uploaded"}"#.to_vec(),
        });
    }

    pub(crate) fn request_object_download(
        &mut self,
        connection_id: u64,
        bucket: String,
        name: String,
    ) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let Some(file_path) = rfd::FileDialog::new().set_file_name(&name).save_file() else {
                return;
            };
            self.backend.send(BackendCommand::DownloadObject {
                connection_id,
                bucket,
                name,
                file_path,
            });
        }

        #[cfg(target_arch = "wasm32")]
        self.backend.send(BackendCommand::DownloadObject {
            connection_id,
            bucket,
            file_path: std::path::PathBuf::from(&name),
            name,
        });
    }
}
