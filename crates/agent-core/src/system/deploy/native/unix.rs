use super::*;
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub(super) async fn bsd_account(ops: &dyn Privileged, name: &str) -> Result<()> {
    if !ops
        .execute(Path::new("pw"), &["groupshow".into(), name.into()])
        .await?
        .success
    {
        command(ops, "pw", &["groupadd".into(), name.into()]).await?;
    }
    if !ops
        .execute(Path::new("pw"), &["usershow".into(), name.into()])
        .await?
        .success
    {
        command(
            ops,
            "pw",
            &[
                "useradd".into(),
                name.into(),
                "-g".into(),
                name.into(),
                "-d".into(),
                "/var/empty".into(),
                "-s".into(),
                "/usr/sbin/nologin".into(),
                "-w".into(),
                "no".into(),
            ],
        )
        .await?;
    }
    Ok(())
}

pub(super) async fn mac_account(ops: &dyn Privileged, name: &str) -> Result<()> {
    if ops
        .execute(
            Path::new("dscl"),
            &[".".into(), "-read".into(), format!("/Users/{name}")],
        )
        .await?
        .success
    {
        return Ok(());
    }
    let users = ops
        .execute(
            Path::new("dscl"),
            &[
                ".".into(),
                "-list".into(),
                "/Users".into(),
                "UniqueID".into(),
            ],
        )
        .await?;
    let groups = ops
        .execute(
            Path::new("dscl"),
            &[
                ".".into(),
                "-list".into(),
                "/Groups".into(),
                "PrimaryGroupID".into(),
            ],
        )
        .await?;
    ensure!(
        users.success && groups.success,
        "cannot inspect system accounts"
    );
    let occupied: std::collections::BTreeSet<u32> = users
        .stdout
        .lines()
        .chain(groups.stdout.lines())
        .filter_map(|line| line.split_whitespace().last()?.parse().ok())
        .collect();
    let id = (300..500)
        .find(|id| !occupied.contains(id))
        .context("no available system account identifier")?
        .to_string();
    for args in [
        vec![".".into(), "-create".into(), format!("/Groups/{name}")],
        vec![
            ".".into(),
            "-create".into(),
            format!("/Groups/{name}"),
            "PrimaryGroupID".into(),
            id.clone(),
        ],
        vec![".".into(), "-create".into(), format!("/Users/{name}")],
    ] {
        command(ops, "dscl", &args).await?;
    }
    for (key, value) in [
        ("UniqueID", id.as_str()),
        ("PrimaryGroupID", id.as_str()),
        ("UserShell", "/usr/bin/false"),
        ("NFSHomeDirectory", "/var/empty"),
        ("IsHidden", "1"),
        ("Password", "*"),
    ] {
        command(
            ops,
            "dscl",
            &[
                ".".into(),
                "-create".into(),
                format!("/Users/{name}"),
                key.into(),
                value.into(),
            ],
        )
        .await?;
    }
    Ok(())
}

async fn service(
    ops: &dyn Privileged,
    backend: ServiceBackend,
    unit: &str,
    args: &[String],
    account: Option<&str>,
    working: &Path,
) -> Result<()> {
    let service = unit.strip_suffix(".service").unwrap_or(unit);
    if backend == ServiceBackend::Launchd {
        let label = format!("org.sinan.{}", service.replace('@', "."));
        let path = PathBuf::from("/Library/LaunchDaemons").join(format!("{label}.plist"));
        let arguments = args
            .iter()
            .map(|value| format!("<string>{}</string>", xml(value)))
            .collect::<String>();
        let account = account
            .map(|value| {
                format!(
                    "<key>UserName</key><string>{}</string><key>GroupName</key><string>{}</string>",
                    xml(value),
                    xml(value)
                )
            })
            .unwrap_or_default();
        let contents = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\"><plist version=\"1.0\"><dict><key>Label</key><string>{label}</string><key>ProgramArguments</key><array>{arguments}</array>{account}<key>WorkingDirectory</key><string>{}</string><key>RunAtLoad</key><true/><key>KeepAlive</key><true/><key>ThrottleInterval</key><integer>5</integer><key>Umask</key><integer>23</integer><key>StandardOutPath</key><string>/var/log/{service}.log</string><key>StandardErrorPath</key><string>/var/log/{service}.log</string></dict></plist>",
            xml(&working.to_string_lossy())
        );
        let loaded = ops
            .execute(
                Path::new("launchctl"),
                &["print".into(), format!("system/{label}")],
            )
            .await?
            .success;
        // Preserve an active runtime; new configuration takes effect at its next explicit restart.
        ops.write_file(&path, contents.as_bytes(), 0o644, None)
            .await?;
        if !loaded {
            command(
                ops,
                "launchctl",
                &[
                    "bootstrap".into(),
                    "system".into(),
                    path.to_string_lossy().into_owned(),
                ],
            )
            .await?;
        }
    } else {
        let name = service.replace(['@', '-', '.'], "_");
        let arguments = args
            .iter()
            .map(|value| quote(value))
            .collect::<Vec<_>>()
            .join(" ");
        let account = account
            .map(|value| format!("-u {} ", quote(value)))
            .unwrap_or_default();
        let script = format!(
            "#!/bin/sh\n# PROVIDE: {name}\n# REQUIRE: NETWORKING\n# KEYWORD: shutdown\n. /etc/rc.subr\nname={name}\nrcvar={name}_enable\npidfile=/var/run/{name}.pid\ncommand=/usr/sbin/daemon\ncommand_args={}\nextra_commands=reload\nreload_cmd={name}_reload\n{name}_reload() {{ kill -HUP \"$(cat /var/run/{name}.child.pid)\"; }}\nstart_precmd={}\nload_rc_config \"$name\"\n: ${{{name}_enable:=NO}}\nrun_rc_command \"$1\"\n",
            quote(&format!(
                "-P /var/run/{name}.pid -p /var/run/{name}.child.pid -r -R 5 -S -T {name} {account}{arguments}"
            )),
            quote(&format!("cd {}", quote(&working.to_string_lossy())))
        );
        ops.write_file(
            &PathBuf::from("/usr/local/etc/rc.d").join(&name),
            script.as_bytes(),
            0o755,
            None,
        )
        .await?;
        command(ops, "sysrc", &[format!("{name}_enable=YES")]).await?;
    }
    Ok(())
}

pub(super) async fn register(
    ops: &dyn Privileged,
    backend: ServiceBackend,
    config: &Config,
    path: &Path,
    binary: &Path,
    descriptor: Option<&Descriptor>,
) -> Result<()> {
    let root = core_root(config)?;
    let mut args = vec![
        binary.to_string_lossy().into_owned(),
        "--config".into(),
        path.to_string_lossy().into_owned(),
        "supervise".into(),
    ];
    if descriptor.is_none() {
        args.push("--monitor-only".into());
    }
    service(ops, backend, "sinan-agent.service", &args, None, &root).await?;
    if let Some(descriptor) = descriptor {
        let runtime = config
            .runtime_root
            .join(format!("{}@main", descriptor.plugin_name));
        let launcher = root.join("runtime-launcher.sh");
        let binary = config
            .install_root
            .join(&descriptor.plugin_name)
            .join("current")
            .join(&descriptor.binary_name);
        let configuration = runtime.join("current/config.json");
        let script = format!(
            "#!/bin/sh\nset -eu\nwhile [ ! -x {} ] || [ ! -f {} ]; do sleep 2; done\nexec {} run -c {} -D {}\n",
            quote(&binary.to_string_lossy()),
            quote(&configuration.to_string_lossy()),
            quote(&binary.to_string_lossy()),
            quote(&configuration.to_string_lossy()),
            quote(&runtime.join("data").to_string_lossy())
        );
        ops.write_file(&launcher, script.as_bytes(), 0o755, None)
            .await?;
        service(
            ops,
            backend,
            &descriptor.service_unit,
            &[launcher.to_string_lossy().into_owned()],
            Some(&descriptor.service_group),
            &runtime.join("data"),
        )
        .await?;
    }
    Ok(())
}
