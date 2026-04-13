use nix::fcntl::OFlag;
use nix::pty::{grantpt, posix_openpt, ptsname, unlockpt};
use rand::Rng;
use std::fs::{self, remove_file};
use std::os::unix::fs::symlink;
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::path::Path;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

struct SerialMock {
    name: String,
    // On utilise Arc<Mutex<File>> pour pouvoir cloner l'accès au port
}

impl SerialMock {
    fn new(path: &str, label: &str) -> Self {
        let master_fd = posix_openpt(OFlag::O_RDWR | OFlag::O_NOCTTY | OFlag::O_NONBLOCK).unwrap();
        grantpt(&master_fd).unwrap();
        unlockpt(&master_fd).unwrap();

        let slave_name = unsafe { ptsname(&master_fd) }.unwrap();
        let link_path = format!("/tmp/{}", path);

        if Path::new(&link_path).exists() {
            let _ = remove_file(&link_path);
        }
        symlink(&slave_name, &link_path).unwrap();

        println!("{} prêt sur : {}", label, link_path);

        let raw_fd = master_fd.into_raw_fd();
        let std_file = unsafe { std::fs::File::from_raw_fd(raw_fd) };
        let tokio_file = File::from_std(std_file);

        Self {
            name: label.to_string(),
            file: Arc::new(Mutex::new(tokio_file)),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let scale = SerialMock::new("ttyPackageHero", "BALANCE");
    let laser_l = SerialMock::new("ttyLaserL", "LASER L");
    let laser_w = SerialMock::new("ttyLaserW", "LASER W");
    let laser_h = SerialMock::new("ttyLaserH", "LASER H");

    let mut buffer_scale = [0; 1024];
    let mut buffer_l = [0; 1024];
    let mut buffer_w = [0; 1024];
    let mut buffer_h = [0; 1024];

    loop {
        let scale_file = scale.file.clone();
        let l_file = laser_l.file.clone();
        let w_file = laser_w.file.clone();
        let h_file = laser_h.file.clone();

        tokio::select! {
            res = async { scale_file.lock().await.read(&mut buffer_scale).await } => {
                if let Ok(n) = res {
                    if n > 0 {
                        let received = &buffer_scale[..n];
                        if received.contains(&0xBB) {
                            println!("Balance : Commande reçue. Envoi ACK (0xAF01)");

                            let mut f = scale_file.lock().await;
                            f.write_all(&[0xAF, 0x01]).await?;
                            f.flush().await?;
                            drop(f);

                            let file_ptr = scale_file.clone();
                            tokio::spawn(async move {
                                tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
                                let poids = rand::thread_rng().gen_range(500..3500);

                                // Format trame balance : A0 0A + Poids ASCII
                                let resp_str = format!("A00A       {}", poids);
                                let mut f = file_ptr.lock().await;
                                let _ = f.write_all(resp_str.as_bytes()).await;
                                let _ = f.write_all(b"\r\n").await;
                                let _ = f.flush().await;
                                println!("Balance : Poids envoyé -> {}g", poids);
                            });
                        }
                    }
                }
            }

            res = async { l_file.lock().await.read(&mut buffer_l).await } => {
                handle_laser(&laser_l.name, l_file, res, &buffer_l).await?;
            }
            res = async { w_file.lock().await.read(&mut buffer_w).await } => {
                handle_laser(&laser_w.name, w_file, res, &buffer_w).await?;
            }
            res = async { h_file.lock().await.read(&mut buffer_h).await } => {
                handle_laser(&laser_h.name, h_file, res, &buffer_h).await?;
            }
        }
    }
}

async fn handle_laser(name: &str, file: Arc<Mutex<File>>, res: io::Result<usize>, buffer: &[u8]) -> io::Result<()> {
    if let Ok(n) = res {
        if n > 0 {
            let received = &buffer[..n];
            if received.iter().any(|&b| b == 0x20) {
                let dist: u32 = rand::thread_rng().gen_range(200..1000);

                // Trame de réponse binaire (14 octets typique)
                let mut resp = vec![0xAA, 0x00, 0x00, 0x22, 0x00, 0x03];
                resp.extend_from_slice(&dist.to_be_bytes()); // Distance 4 octets
                resp.extend_from_slice(&[0x00, 0x30, 0xEE, 0x0D, 0x0A]); // Qualité + Checksum + Fin

                let mut f = file.lock().await;
                f.write_all(&resp).await?;
                f.flush().await?;
                println!("{}: {}mm envoyé", name, dist);
            }
        }
    }
    Ok(())
}