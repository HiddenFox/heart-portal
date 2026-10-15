# Android relay reconnect control

The Android shell currently launches Portal with `ProcessBuilder`, merges stdout and stderr for lifecycle parsing, and does not keep or write the child stdin stream. Portal `main.rs` does not read stdin as a control protocol. Therefore this release intentionally does not add a process-kill reconnect path.

The next compatible control protocol should use a line-delimited stdin command, `{"type":"relay_reconnect_now"}\n`. The Rust owner must acknowledge it only through a redacted lifecycle log, coalesce duplicate requests, and cause the active relay session to return so the existing reconnect loop reconnects without consuming Android crash-restart budget. Kotlin should write the command only while parsed relay state has remained `RETRYING` for its debounce period, and must rate-limit requests to one per 30 seconds.

Until that protocol exists, Android observes validated-network recovery and reports it diagnostically; Rust's normal backoff remains the only reconnect mechanism. This avoids interrupting in-flight MCP work or creating a restart loop.
