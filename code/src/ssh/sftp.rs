use ssh2::{Session, Sftp};
use std::path::{Path, PathBuf};
use std::io::{Read, Write, BufReader, BufWriter};
use std::fs::File;
use std::net::TcpStream;
use std::time::Duration;
use anyhow::{Result, Context};

use crate::ssh::config::{SshConfig, AuthMethod};

/// SFTP 文件条目信息
#[derive(Clone, Debug)]
pub struct SftpEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<std::time::SystemTime>,
}

impl SftpEntry {
    /// 获取文件大小显示字符串
    pub fn size_display(&self) -> String {
        if self.is_dir {
            return String::from("<DIR>");
        }

        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if self.size >= GB {
            format!("{:.1} GB", self.size as f64 / GB as f64)
        } else if self.size >= MB {
            format!("{:.1} MB", self.size as f64 / MB as f64)
        } else if self.size >= KB {
            format!("{:.1} KB", self.size as f64 / KB as f64)
        } else {
            format!("{} B", self.size)
        }
    }
}

/// SFTP 文件管理器
pub struct SftpManager {
    sftp: Sftp,
    current_path: PathBuf,
    home_path: PathBuf,
}

impl SftpManager {
    /// 连接到 SSH 服务器并创建 SFTP 管理器
    pub fn connect(config: &SshConfig) -> Result<Self> {
        let addr = format!("{}:{}", config.host, config.port);

        // 建立 TCP 连接
        let tcp = TcpStream::connect_timeout(
            &addr.parse().context("Invalid address")?,
            Duration::from_secs(config.timeout_secs),
        ).context("Failed to connect")?;

        // 创建 SSH 会话
        let mut session = Session::new().context("Failed to create SSH session")?;
        session.set_tcp_stream(tcp);
        session.handshake().context("SSH handshake failed")?;

        // 认证
        match &config.auth {
            AuthMethod::Password(password) => {
                session.userauth_password(&config.username, password)
                    .context("Password authentication failed")?;
            }
            AuthMethod::KeyFile(key_file) => {
                session.userauth_pubkey_file(
                    &config.username,
                    None,
                    key_file,
                    config.key_passphrase.as_deref(),
                ).context("SSH key authentication failed")?;
            }
        }

        if !session.authenticated() {
            anyhow::bail!("SSH authentication failed");
        }

        Self::new(&session)
    }

    /// 创建新的 SFTP 管理器
    pub fn new(session: &Session) -> Result<Self> {
        let sftp = session.sftp()
            .context("Failed to open SFTP channel")?;

        // 获取用户主目录作为起始路径
        let home_path = sftp.realpath(Path::new("."))
            .unwrap_or_else(|_| PathBuf::from("/"));
        let current_path = home_path.clone();

        Ok(Self {
            sftp,
            current_path,
            home_path,
        })
    }

    /// 获取当前路径
    pub fn current_path(&self) -> &Path {
        &self.current_path
    }

    /// 列出当前目录内容
    pub fn list_dir(&self) -> Result<Vec<SftpEntry>> {
        self.list_path(&self.current_path)
    }

    /// 列出指定目录内容
    pub fn list_path(&self, path: &Path) -> Result<Vec<SftpEntry>> {
        let mut entries = Vec::new();

        let dir = self.sftp.readdir(path)
            .with_context(|| format!("Failed to read directory: {:?}", path))?;

        for (path_buf, stat) in dir {
            let name = path_buf.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            // 跳过 . 和 ..
            if name == "." || name == ".." {
                continue;
            }

            let is_dir = stat.is_dir();
            let size = stat.size.unwrap_or(0);
            let modified = stat.mtime.map(|t| {
                std::time::UNIX_EPOCH + std::time::Duration::from_secs(t as u64)
            });

            entries.push(SftpEntry {
                name,
                path: path_buf,
                is_dir,
                size,
                modified,
            });
        }

        // 排序：目录优先，然后按名称排序
        entries.sort_by(|a, b| {
            if a.is_dir != b.is_dir {
                b.is_dir.cmp(&a.is_dir)
            } else {
                a.name.to_lowercase().cmp(&b.name.to_lowercase())
            }
        });

        Ok(entries)
    }

    /// 切换到子目录
    pub fn cd(&mut self, name: &str) -> Result<()> {
        let new_path = self.current_path.join(name);
        self.cd_path(&new_path)
    }

    /// 切换到指定路径
    pub fn cd_path(&mut self, path: &Path) -> Result<()> {
        // 验证路径存在且是目录
        let stat = self.sftp.stat(path)
            .with_context(|| format!("Path not found: {:?}", path))?;

        if !stat.is_dir() {
            anyhow::bail!("Not a directory: {:?}", path);
        }

        // 解析绝对路径
        let real_path = self.sftp.realpath(path)
            .with_context(|| format!("Failed to resolve path: {:?}", path))?;

        self.current_path = real_path;
        Ok(())
    }

    /// 返回上级目录
    pub fn cd_up(&mut self) -> Result<()> {
        if let Some(parent) = self.current_path.parent() {
            self.current_path = parent.to_path_buf();
        }
        Ok(())
    }

    /// 返回主目录
    pub fn cd_home(&mut self) -> Result<()> {
        let home = self.home_path.clone();
        self.cd_path(&home)
    }

    /// 获取主目录路径
    pub fn home_path(&self) -> &Path {
        &self.home_path
    }

    /// 下载文件
    pub fn download(&self, remote_path: &Path, local_path: &Path) -> Result<()> {
        let mut remote_file = self.sftp.open(remote_path)
            .with_context(|| format!("Failed to open remote file: {:?}", remote_path))?;

        let mut local_file = File::create(local_path)
            .with_context(|| format!("Failed to create local file: {:?}", local_path))?;

        let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer
        loop {
            let bytes_read = remote_file.read(&mut buffer)
                .context("Failed to read from remote file")?;
            if bytes_read == 0 {
                break;
            }
            local_file.write_all(&buffer[..bytes_read])
                .context("Failed to write to local file")?;
        }

        Ok(())
    }

    /// 下载文件夹（递归）
    pub fn download_dir(&self, remote_path: &Path, local_path: &Path) -> Result<()> {
        // 创建本地目录
        std::fs::create_dir_all(local_path)
            .with_context(|| format!("Failed to create local directory: {:?}", local_path))?;

        // 列出远程目录内容
        let entries = self.list_path(remote_path)
            .with_context(|| format!("Failed to list remote directory: {:?}", remote_path))?;

        for entry in entries {
            let remote_entry_path = entry.path.clone();
            let local_entry_path = local_path.join(&entry.name);

            if entry.is_dir {
                // 递归下载子目录
                self.download_dir(&remote_entry_path, &local_entry_path)?;
            } else {
                // 下载文件
                self.download(&remote_entry_path, &local_entry_path)?;
            }
        }

        Ok(())
    }

    /// 上传文件
    pub fn upload(&self, local_path: &Path, remote_path: &Path) -> Result<()> {
        let mut local_file = File::open(local_path)
            .with_context(|| format!("Failed to open local file: {:?}", local_path))?;

        let remote_file = self.sftp.create(remote_path)
            .with_context(|| format!("Failed to create remote file: {:?}", remote_path))?;

        let mut writer = BufWriter::new(remote_file);
        let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer

        loop {
            let bytes_read = local_file.read(&mut buffer)
                .context("Failed to read from local file")?;
            if bytes_read == 0 {
                break;
            }
            writer.write_all(&buffer[..bytes_read])
                .context("Failed to write to remote file")?;
        }

        Ok(())
    }

    /// 删除文件
    pub fn delete_file(&self, path: &Path) -> Result<()> {
        self.sftp.unlink(path)
            .with_context(|| format!("Failed to delete file: {:?}", path))
    }

    /// 删除目录
    pub fn delete_dir(&self, path: &Path) -> Result<()> {
        self.sftp.rmdir(path)
            .with_context(|| format!("Failed to delete directory: {:?}", path))
    }

    /// 重命名
    pub fn rename(&self, old_path: &Path, new_path: &Path) -> Result<()> {
        self.sftp.rename(old_path, new_path, None)
            .with_context(|| format!("Failed to rename: {:?} -> {:?}", old_path, new_path))
    }

    /// 创建目录
    pub fn mkdir(&self, path: &Path) -> Result<()> {
        self.sftp.mkdir(path, 0o755)
            .with_context(|| format!("Failed to create directory: {:?}", path))
    }

    /// 检查路径是否存在
    pub fn exists(&self, path: &Path) -> bool {
        self.sftp.stat(path).is_ok()
    }

    /// 获取默认下载目录
    pub fn default_download_dir() -> PathBuf {
        dirs::download_dir()
            .or_else(|| dirs::home_dir())
            .unwrap_or_else(|| PathBuf::from("."))
    }
}