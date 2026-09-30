use super::*;
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
pub(super) async fn powershell(ops: &dyn Privileged, script: &str) -> Result<()> {
    command(
        ops,
        "powershell.exe",
        &[
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-Command".into(),
            format!("$ErrorActionPreference='Stop'; {script}"),
        ],
    )
    .await
}

pub(super) async fn account(ops: &dyn Privileged, name: &str) -> Result<()> {
    let script = format!(
        "if (-not (Get-LocalUser -Name {} -ErrorAction SilentlyContinue)) {{ New-LocalUser -Name {} -NoPassword -AccountNeverExpires -UserMayNotChangePassword | Out-Null }}",
        quote(name),
        quote(name)
    );
    powershell(ops, &script).await
}

async fn task(
    ops: &dyn Privileged,
    name: &str,
    program: &Path,
    args: &str,
    account: Option<&str>,
    directory: &Path,
) -> Result<()> {
    let principal = if let Some(account) = account {
        format!(
            "$random=New-Object byte[] 48; [Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($random); $password=[Convert]::ToBase64String($random); Set-LocalUser -Name {} -Password (ConvertTo-SecureString $password -AsPlainText -Force) -PasswordNeverExpires $true; Register-ScheduledTask -TaskName {} -Action $action -Trigger $trigger -Settings $settings -User {} -Password $password -RunLevel Limited -Force | Out-Null",
            quote(account),
            quote(name),
            quote(account)
        )
    } else {
        format!(
            "$principal=New-ScheduledTaskPrincipal -UserId 'SYSTEM' -LogonType ServiceAccount -RunLevel Highest; Register-ScheduledTask -TaskName {} -Action $action -Trigger $trigger -Settings $settings -Principal $principal -Force | Out-Null",
            quote(name)
        )
    };
    let script = format!(
        "$action=New-ScheduledTaskAction -Execute {} -Argument {} -WorkingDirectory {}; $trigger=New-ScheduledTaskTrigger -AtStartup; $settings=New-ScheduledTaskSettingsSet -StartWhenAvailable -ExecutionTimeLimit ([TimeSpan]::Zero) -RestartCount 999 -RestartInterval (New-TimeSpan -Minutes 1) -MultipleInstances IgnoreNew -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries; {principal}",
        quote(&program.to_string_lossy()),
        quote(args),
        quote(&directory.to_string_lossy())
    );
    powershell(ops, &script).await
}

pub(super) async fn register(
    ops: &dyn Privileged,
    config: &Config,
    path: &Path,
    binary: &Path,
    descriptor: Option<&Descriptor>,
) -> Result<()> {
    let root = core_root(config)?;
    ensure!(
        !path.to_string_lossy().contains('"'),
        "configuration path cannot contain a quote"
    );
    let args = format!(
        "--config \"{}\" supervise{}",
        path.display(),
        if descriptor.is_none() {
            " --monitor-only"
        } else {
            ""
        }
    );
    task(ops, "sinan-agent", binary, &args, None, &root).await?;
    if let Some(descriptor) = descriptor {
        let runtime = config
            .runtime_root
            .join(format!("{}@main", descriptor.plugin_name));
        let script = root.join("runtime-launcher.ps1");
        let contents = format!(
            r#"$ErrorActionPreference='Stop'
$kernel={}
$configuration={}
while ($true) {{
    if ((Test-Path -LiteralPath $kernel) -and (Test-Path -LiteralPath $configuration)) {{
        $k=Get-Content -LiteralPath $kernel -Raw | ConvertFrom-Json
        $c=Get-Content -LiteralPath $configuration -Raw | ConvertFrom-Json
        if (-not $k.sinan_directory_reference -or -not $c.sinan_directory_reference) {{ throw 'Invalid runtime reference' }}
        $binary=Join-Path $k.target {}
        $config=Join-Path $c.target 'config.json'
        & $binary run -c $config -D {}
    }}
    Start-Sleep -Seconds 5
}}
"#,
            quote(
                &config
                    .install_root
                    .join(&descriptor.plugin_name)
                    .join("current")
                    .to_string_lossy()
            ),
            quote(&runtime.join("current").to_string_lossy()),
            quote(&descriptor.binary_name),
            quote(&runtime.join("data").to_string_lossy())
        );
        ops.write_file(&script, contents.as_bytes(), 0o755, None)
            .await?;
        let name = descriptor
            .service_unit
            .strip_suffix(".service")
            .unwrap_or(&descriptor.service_unit);
        task(
            ops,
            name,
            Path::new("powershell.exe"),
            &format!(
                "-NoProfile -NonInteractive -ExecutionPolicy Bypass -File \"{}\"",
                script.display()
            ),
            Some(&descriptor.service_group),
            &runtime.join("data"),
        )
        .await?;
    }
    Ok(())
}
