use ssh2::{Session, Sftp};
use std::path::{Path, PathBuf};
use std::io::{Read, Write, BufReader, BufWriter, Seek, SeekFrom};
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

/// 进度回调类型
pub type ProgressCallback = Box<dyn Fn(u64, u64) + Send>;  // (已传输字节数, 总字节数)

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

    /// 下载文件（带进度回调）
    pub fn download_with_progress(&self, remote_path: &Path, local_path: &Path, progress: Option<ProgressCallback>) -> Result<()> {
        let mut remote_file = self.sftp.open(remote_path)
            .with_context(|| format!("Failed to open remote file: {:?}", remote_path))?;

        // 获取文件大小
        let stat = self.sftp.stat(remote_path)
            .with_context(|| format!("Failed to stat remote file: {:?}", remote_path))?;
        let total_size = stat.size.unwrap_or(0);

        let mut local_file = File::create(local_path)
            .with_context(|| format!("Failed to create local file: {:?}", local_path))?;

        let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer
        let mut transferred = 0u64;

        loop {
            let bytes_read = remote_file.read(&mut buffer)
                .context("Failed to read from remote file")?;
            if bytes_read == 0 {
                break;
            }
            local_file.write_all(&buffer[..bytes_read])
                .context("Failed to write to local file")?;

            transferred += bytes_read as u64;
            if let Some(ref cb) = progress {
                cb(transferred, total_size);
            }
        }

        Ok(())
    }

    /// 下载文件
    pub fn download(&self, remote_path: &Path, local_path: &Path) -> Result<()> {
        self.download_with_progress(remote_path, local_path, None)
    }

    /// 下载文件夹（递归，带进度回调）
    pub fn download_dir_with_progress(&self, remote_path: &Path, local_path: &Path, progress: Option<&dyn Fn(u64, u64)>) -> Result<u64> {
        // 创建本地目录
        std::fs::create_dir_all(local_path)
            .with_context(|| format!("Failed to create local directory: {:?}", local_path))?;

        // 列出远程目录内容
        let entries = self.list_path(remote_path)
            .with_context(|| format!("Failed to list remote directory: {:?}", remote_path))?;

        let mut total_transferred = 0u64;

        for entry in entries {
            let remote_entry_path = entry.path.clone();
            let local_entry_path = local_path.join(&entry.name);

            if entry.is_dir {
                // 递归下载子目录
                let transferred = self.download_dir_with_progress(&remote_entry_path, &local_entry_path, progress)?;
                total_transferred += transferred;
            } else {
                // 下载文件
                self.download_file_with_progress(&remote_entry_path, &local_entry_path, progress)?;
                total_transferred += entry.size;
            }
        }

        Ok(total_transferred)
    }

    /// 下载文件（带进度回调）- 内部方法
    fn download_file_with_progress(&self, remote_path: &Path, local_path: &Path, progress: Option<&dyn Fn(u64, u64)>) -> Result<()> {
        let mut remote_file = self.sftp.open(remote_path)
            .with_context(|| format!("Failed to open remote file: {:?}", remote_path))?;

        // 获取文件大小
        let stat = self.sftp.stat(remote_path)
            .with_context(|| format!("Failed to stat remote file: {:?}", remote_path))?;
        let total_size = stat.size.unwrap_or(0);

        let mut local_file = File::create(local_path)
            .with_context(|| format!("Failed to create local file: {:?}", local_path))?;

        let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer
        let mut transferred = 0u64;

        loop {
            let bytes_read = remote_file.read(&mut buffer)
                .context("Failed to read from remote file")?;
            if bytes_read == 0 {
                break;
            }
            local_file.write_all(&buffer[..bytes_read])
                .context("Failed to write to local file")?;

            transferred += bytes_read as u64;
            if let Some(cb) = progress {
                cb(transferred, total_size);
            }
        }

        Ok(())
    }

    /// 下载文件夹（递归）
    pub fn download_dir(&self, remote_path: &Path, local_path: &Path) -> Result<()> {
        self.download_dir_with_progress(remote_path, local_path, None)?;
        Ok(())
    }

    /// 上传文件（带进度回调）
    pub fn upload_with_progress(&self, local_path: &Path, remote_path: &Path, progress: Option<ProgressCallback>) -> Result<()> {
        self.upload_file_with_progress(local_path, remote_path, progress.as_ref().map(|p| p as &dyn Fn(u64, u64)))
    }

    /// 上传文件（带进度回调）- 内部方法
    fn upload_file_with_progress(&self, local_path: &Path, remote_path: &Path, progress: Option<&dyn Fn(u64, u64)>) -> Result<()> {
        let mut local_file = File::open(local_path)
            .with_context(|| format!("无法打开本地文件: {:?}", local_path))?;

        // 获取本地文件大小
        let total_size = local_file.metadata()
            .with_context(|| format!("无法获取文件信息: {:?}", local_path))?
            .len();

        let remote_file = self.sftp.create(remote_path)
            .with_context(|| format!("无法创建远程文件: {:?}", remote_path))?;

        let mut writer = BufWriter::new(remote_file);
        let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer
        let mut transferred = 0u64;

        loop {
            let bytes_read = local_file.read(&mut buffer)
                .context("读取本地文件失败")?;
            if bytes_read == 0 {
                break;
            }
            writer.write_all(&buffer[..bytes_read])
                .context("写入远程文件失败")?;

            transferred += bytes_read as u64;
            if let Some(cb) = progress {
                cb(transferred, total_size);
            }
        }

        // 刷新缓冲区确保所有数据写入
        writer.flush()
            .context("刷新写入缓冲区失败")?;

        Ok(())
    }

    /// 上传文件
    pub fn upload(&self, local_path: &Path, remote_path: &Path) -> Result<()> {
        self.upload_with_progress(local_path, remote_path, None)
    }

    /// 上传文件夹（递归，带进度回调）
    pub fn upload_dir_with_progress(&self, local_path: &Path, remote_path: &Path, progress: Option<&dyn Fn(u64, u64)>) -> Result<u64> {
        // 创建远程目录
        self.mkdir(remote_path).ok(); // 忽略已存在错误

        // 遍历本地目录
        let entries = std::fs::read_dir(local_path)
            .with_context(|| format!("Failed to read local directory: {:?}", local_path))?;

        let mut total_transferred = 0u64;

        for entry in entries {
            let entry = entry.context("Failed to read directory entry")?;
            let local_entry_path = entry.path();
            let remote_entry_path = remote_path.join(entry.file_name());

            let file_type = entry.file_type()
                .with_context(|| format!("Failed to get file type: {:?}", local_entry_path))?;

            if file_type.is_dir() {
                // 递归上传子目录
                let transferred = self.upload_dir_with_progress(&local_entry_path, &remote_entry_path, progress)?;
                total_transferred += transferred;
            } else if file_type.is_file() {
                // 上传文件
                let size = entry.metadata()
                    .with_context(|| format!("Failed to get file metadata: {:?}", local_entry_path))?
                    .len();
                self.upload_file_with_progress(&local_entry_path, &remote_entry_path, progress)?;
                total_transferred += size;
            }
        }

        Ok(total_transferred)
    }

    /// 上传文件夹（递归）
    pub fn upload_dir(&self, local_path: &Path, remote_path: &Path) -> Result<()> {
        self.upload_dir_with_progress(local_path, remote_path, None)?;
        Ok(())
    }

    /// 删除文件
    pub fn delete_file(&self, path: &Path) -> Result<()> {
        self.sftp.unlink(path)
            .with_context(|| format!("删除文件失败: {:?}", path))
    }

    /// 计算目录总大小（用于进度计算）
    pub fn dir_size(&self, path: &Path) -> Result<u64> {
        let entries = self.list_path(path)
            .with_context(|| format!("无法列出目录: {:?}", path))?;

        let mut total = 0u64;
        for entry in entries {
            if entry.is_dir {
                total += self.dir_size(&entry.path)?;
            } else {
                total += entry.size;
            }
        }
        Ok(total)
    }

    /// 删除目录（递归删除非空目录，带进度回调）
    pub fn delete_dir_with_progress(&self, path: &Path, progress: Option<&dyn Fn(u64, u64)>) -> Result<u64> {
        // 获取总大小用于进度计算
        let total_size = self.dir_size(path).ok().unwrap_or(0);
        let mut deleted_size = 0u64;

        // 先删除目录内容
        let entries = self.list_path(path)
            .with_context(|| format!("无法列出目录: {:?}", path))?;

        for entry in entries {
            let entry_path = entry.path.clone();
            if entry.is_dir {
                let size = self.delete_dir_with_progress(&entry_path, progress)?;
                deleted_size += size;
            } else {
                self.delete_file(&entry_path)?;
                deleted_size += entry.size;
            }

            if let Some(cb) = progress {
                cb(deleted_size, total_size);
            }
        }

        // 删除空目录
        self.sftp.rmdir(path)
            .with_context(|| format!("删除目录失败: {:?}", path))?;

        Ok(deleted_size)
    }

    /// 删除目录（递归删除非空目录）
    pub fn delete_dir(&self, path: &Path) -> Result<()> {
        self.delete_dir_with_progress(path, None)?;
        Ok(())
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