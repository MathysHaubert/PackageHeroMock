use nix::fcntl::OFlag;
use nix::pty::{grantpt, posix_openpt, ptsname, unlockpt};
use rand::Rng;
use std::fs::remove_file;
use std::os::unix::fs::symlink;
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::path::Path;
use std::time::Duration;
use tokio::io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let master_fd = posix_openpt(OFlag::O_RDWR | OFlag::O_NOCTTY | OFlag::O_NONBLOCK)?;
    grantpt(&master_fd)?;
    unlockpt(&master_fd)?;

    let slave_name = unsafe { ptsname(&master_fd) }?;
    let link_path = "/tmp/ttyPackageHero";

    if Path::new(link_path).exists() {
        remove_file(link_path)?;
    }
    symlink(&slave_name, link_path)?;

    println!("PackageHERO Simulator ready on: {}", link_path);
    println!("Press ENTER to send weight (Scale)");
    println!("Press 'L' then ENTER for measurement (Laser)");

    let raw_fd = master_fd.into_raw_fd();
    let std_file = unsafe { std::fs::File::from_raw_fd(raw_fd) };
    let mut tokio_file = io::BufStream::new(tokio::fs::File::from_std(std_file));

    let mut reader_stdin = BufReader::new(io::stdin()).lines();

    tokio::time::sleep(Duration::from_secs(1)).await;
    let _ = tokio_file.write_all(b"AF01\r\n").await;
    let _ = tokio_file.write_all(&[0x00, 0x00, 0x0D, 0x0A]).await;
    let _ = tokio_file.flush().await;

    let mut buffer = [0; 1024];

    loop {
        tokio::select! {
                    line_result = reader_stdin.next_line() => {
                        if let Ok(Some(input)) = line_result {
                            match input.to_uppercase().as_str() {
                                "L" => {
                                    let dist: u16 = rand::thread_rng().gen_range(500..2000);
                                    let hi = (dist >> 8) as u8;
                                    let lo = (dist & 0xFF) as u8;
                                    let resp = vec![0xAA, 0x00, 0x00, 0x22, 0x00, hi, lo, 0x00, 0x80, 0x0D, 0x0A];

                                    println!("Laser: Sending distance {}mm", dist);
                                    let _ = tokio_file.write_all(&resp).await;
                                    let _ = tokio_file.flush().await;
                                    println!("Laser sent.");                        }
                                _ => {
                                    let poids = rand::thread_rng().gen_range(100..5000);
                                    let reponse = format!("A00A{:010}\r\n", poids);
                                    println!("Scale: Sending weight {}g", poids);
                                    let _ = tokio_file.write_all(reponse.as_bytes()).await;
                                    let _ = tokio_file.flush().await;
                                    println!("Weight sent.");
                                }
                            }
                        }
                    }

                    result = tokio_file.read(&mut buffer) => {
                        let n = match result {
                            Ok(n) => n,
                            Err(_) => continue,
                        };
                        if n == 0 { continue; }
                        let received = &buffer[..n];

                        if received.starts_with(&[0xAA]) {
                            let ack_laser = vec![0xAA, 0x01, 0xBE, 0x00, 0x01, 0x00, 0x01, 0xC1, 0x0D, 0x0A];
                            let _ = tokio_file.write_all(&ack_laser).await;
                            let _ = tokio_file.flush().await;
                        } else {
                            let cmd = String::from_utf8_lossy(received).trim().to_string();
                            match cmd.as_str() {
                                "BB100000" => { let _ = tokio_file.write_all(b"AF01\r\n").await; }
                                "BB010000" => { let _ = tokio_file.write_all(format!("A00A{:010}\r\n", 1250).as_bytes()).await; }
                                "BB040000" => { let _ = tokio_file.write_all(b"AD12AD22AD32AD42\r\n").await; }
                                _ => {}
                            }
                            let _ = tokio_file.flush().await;
                        }
                    }
                }
    }
}
