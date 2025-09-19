This library provides traits that mirror the APIs provided by
[`tokio::process`][].

In addition to those traits, it provides two implementations: one backed by
tokio, and a mock implementation.

This is useful for unit testing code that spawns child processes, as it often
would not be safe or possible to spawn those child processes inside a unit test.
Tokio does not provide a first-party solution for mocking these APIs.

To use this, functions that would normally use [`tokio::process`][] need to be
generic over the `CommandSpawner` trait. Then, these functions can be called
with either a real tokio command spawner, or a mock spawner.

```rust
use async_process_traits::Command;
use async_process_traits::Child;
use async_process_traits::CommandSpawner;
use async_process_traits::TokioCommandSpawner;
use tokio::io;

async fn run_command<Spwn: CommandSpawner>(spawner: Spwn) -> io::Result<()> {
    let mut cmd = Spwn::Command::new("echo");
    cmd.args(&["Hello, world!"]);

    let mut child = spawner.spawn(&mut cmd)?;
    child.wait().await?;

    Ok(())
}

#[tokio::main]
async fn main() -> io::Result<()> {
    run_command(TokioCommandSpawner).await
}

#[cfg(test)]
mod tests {
    use async_process_traits::MockCommandSpawner;

    use super::*;

    #[tokio::test]
    async fn test_run_command() {
        run_command(MockCommandSpawner).unwrap();
    }
}
```

[`tokio::process`]: https://docs.rs/tokio/latest/tokio/process/
