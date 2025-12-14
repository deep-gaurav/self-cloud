use crate::common::FileInfo;
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use crate::api::user;

#[server(ListFiles, "/api/files")]
pub async fn list_files(path: String) -> Result<Vec<FileInfo>, ServerFnError> {
    user()?;
    let path = if path.is_empty() {
        std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
    } else {
        path
    };

    let paths = std::fs::read_dir(&path).map_err(|e| ServerFnError::new(e.to_string()))?;
    let mut files = Vec::new();
    for entry in paths {
        if let Ok(entry) = entry {
            if let Ok(meta) = entry.metadata() {
                files.push(FileInfo {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: entry.path().to_string_lossy().to_string(),
                    is_dir: meta.is_dir(),
                    size: meta.len(),
                    modified: meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs()),
                });
            }
        }
    }

    files.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    Ok(files)
}

#[server(ReadFile, "/api/files/read")]
pub async fn read_file(path: String) -> Result<String, ServerFnError> {
    user()?;
    let content = std::fs::read_to_string(&path).map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(content)
}

#[server(WriteFile, "/api/files/write")]
pub async fn write_file(path: String, content: String) -> Result<(), ServerFnError> {
    user()?;
    std::fs::write(&path, content).map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server(DeleteFile, "/api/files/delete")]
pub async fn delete_file(path: String) -> Result<(), ServerFnError> {
    user()?;
    let meta = std::fs::metadata(&path).map_err(|e| ServerFnError::new(e.to_string()))?;
    if meta.is_dir() {
        std::fs::remove_dir_all(&path).map_err(|e| ServerFnError::new(e.to_string()))?;
    } else {
        std::fs::remove_file(&path).map_err(|e| ServerFnError::new(e.to_string()))?;
    }
    Ok(())
}
