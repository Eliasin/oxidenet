use std::fmt::Display;

use log::error;
use smol::future::race;
use smol::io::{AsyncBufReadExt, BufReader};

pub enum InputConsumptionResult {
    Continue,
    TerminateCommand { reason: String },
}

pub async fn watch<
    I: Display,
    T: FnMut(&str) -> InputConsumptionResult,
    U: FnMut(&str) -> InputConsumptionResult,
>(
    command_identifier: I,
    mut command: smol::process::Command,
    mut stdout_consumer: T,
    mut stderr_consumer: U,
) {
    let mut handle = command.spawn().unwrap();

    let stdout = handle.stdout.take().unwrap();
    let stderr = handle.stderr.take().unwrap();

    let mut stdout_buf_reader = BufReader::new(stdout);
    let mut stderr_buf_reader = BufReader::new(stderr);

    loop {
        if let Err(err) = race::<anyhow::Result<()>, _, _>(
            async {
                let mut line = String::new();
                stdout_buf_reader.read_line(&mut line).await?;
                if let InputConsumptionResult::TerminateCommand { reason } = stdout_consumer(&line)
                {
                    Err(anyhow::anyhow!(
                        "Ending command after stderr output: {reason}"
                    ))
                } else {
                    Ok(())
                }
            },
            async {
                let mut line = String::new();
                stderr_buf_reader.read_line(&mut line).await?;
                if let InputConsumptionResult::TerminateCommand { reason } = stderr_consumer(&line)
                {
                    Err(anyhow::anyhow!(
                        "Ending command after stderr output: {reason}"
                    ))
                } else {
                    Ok(())
                }
            },
        )
        .await
        {
            error!("Watch command {command_identifier} stopped due to {err}");
            break;
        }
    }
}
