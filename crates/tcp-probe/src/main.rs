#![forbid(unsafe_code)]

use sinan_tcp_probe::{Command, Journal, VERSION, parse, run};
use std::{io::Write, time::Duration};

fn main() {
    let command = match parse(std::env::args().skip(1)) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("TCP 诊断参数无效：{error}");
            std::process::exit(2);
        }
    };
    match command {
        Command::Help => {
            println!(
                "原生 TCP 连接诊断（无排名、上传或测速）\n用法：sinan-tcp-probe --workspace <绝对私有目录> --targets <目录内快照文件名> --target-digest <SHA256> --ip-version <4|6> --count <4|8> --concurrency <1|2> --no-rank-upload\n默认 count=4、concurrency=1；最多8目标，总60秒含排队，预留2秒保存。只建立并关闭TCP连接，不发送应用数据。地区/运营商为配置标签；结果不是包丢失率。--version 查看版本。\n本工具尚未接入面板/Agent，不接受宿主改动、raw socket、测速或任意选项。"
            );
        }
        Command::Version => println!("sinan-tcp-probe {VERSION}"),
        Command::Run(options) => {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .max_blocking_threads(4)
                .build()
                .expect("create probe runtime");
            let result = runtime.block_on(async {
                let mut journal = Journal::open(&options).await?;
                run(&options, &mut journal).await
            });
            // A timed-out system resolver must not delay process exit indefinitely.
            runtime.shutdown_timeout(Duration::from_millis(100));
            match result {
                Ok(report) => {
                    let bytes = serde_json::to_vec(&report).expect("serialize bounded report");
                    if bytes.len() > sinan_tcp_probe::OUTPUT_LIMIT
                        || std::io::stdout()
                            .write_all(&bytes)
                            .and_then(|_| std::io::stdout().write_all(b"\n"))
                            .is_err()
                    {
                        std::process::exit(1);
                    }
                    if !report.complete {
                        std::process::exit(1);
                    }
                }
                Err(error) => {
                    eprintln!("TCP 诊断失败：{error}");
                    std::process::exit(1);
                }
            }
        }
    }
}
