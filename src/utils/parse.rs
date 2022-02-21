//! parse
//! It is a module that parses and executes commands.

/// Collection of functions related to shell commands and processes.
pub mod shell_command {
    use crate::prelude::*;
    use anyhow::Error as AnyhowError;

    use async_trait::async_trait;

    use smol::process::{Child as SmolChild, Command as SmolCommand};

    use std::collections::LinkedList;
    use std::convert::AsRef;
    use std::ffi::OsStr;
    use std::fs::{File, OpenOptions};
    use std::iter::Iterator;
    use std::mem;
    use std::ops::{Deref, DerefMut};
    use std::path::Path;
    use std::process::{Child as StdChild, Command, Output, Stdio};

    /// The linkedlist of ChildGuard.
    pub type ChildGuardList<T> = LinkedList<ChildGuard<T>>;

    macro_rules! impl_command_unify{
        ($($command:ty => $child:ty),+) => {
            $(impl CommandUnify<$child> for $command {
                fn new<S: AsRef<OsStr>>(program: S) -> Self {
                    Self::new(program.as_ref())
                }
                fn args<I, S>(&mut self, args: I) -> &mut Self
                where
                    I: IntoIterator<Item = S>,
                    S: AsRef<OsStr>,
                {
                    self.args(args);
                    self
                }

                fn stdin<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Self {
                    self.stdin(cfg);
                    self
                }

                fn stdout<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Self {
                    self.stdout(cfg);
                    self
                }

                fn stderr<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Self {
                    self.stderr(cfg);
                    self
                }

                fn spawn(&mut self) -> AnyResult<$child> {
                    Ok(self.spawn()?)
                }
            })+
        }
    }

    /// Abstraction of command methods in multiple libraries.
    pub trait CommandUnify<Child: ChildUnify>: Sized {
        /// Constructs a new Command for launching the program at path program.
        fn new<S: AsRef<OsStr>>(program: S) -> Self;

        /// Adds multiple arguments to pass to the program.
        fn args<I, S>(&mut self, args: I) -> &mut Self
        where
            I: IntoIterator<Item = S>,
            S: AsRef<OsStr>;

        /// Configuration for the child process's standard input (stdin) handle.
        fn stdin<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Self;

        /// Configuration for the child process's standard output (stdout) handle.
        fn stdout<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Self;

        /// Configuration for the child process's standard error (stderr) handle.
        fn stderr<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Self;

        /// Executes the command as a child process, returning a handle to it.
        fn spawn(&mut self) -> AnyResult<Child>;
    }

    impl_command_unify!(Command => StdChild,SmolCommand => SmolChild);

    use std::convert::TryInto;
    use tokio::process::Child as TokioChild;
    use tokio::process::Command as TokioCommand;
    impl_command_unify!(TokioCommand => TokioChild);

    #[async_trait]
    /// Trait abstraction of multiple library process handles.
    pub trait ChildUnify: Send + Sync {
        /// Executes the command as a child process, waiting for it to finish and returning the status that it exited with.
        async fn wait(self) -> AnyResult<ExitStatus>;

        /// Executes the command as a child process, waiting for it to finish and collecting all of its output.
        async fn wait_with_output(self) -> AnyResult<Output>;
        /// Convert stdout to stdio.
        async fn stdout_to_stdio(&mut self) -> Option<Stdio>;

        /// Kill the process child.
        fn kill(&mut self) -> AnyResult<()>;
    }

    #[async_trait]
    impl ChildUnify for StdChild {
        async fn wait(self) -> AnyResult<ExitStatus> {
            Ok(self.wait().await?)
        }
        async fn wait_with_output(self) -> AnyResult<Output> {
            Ok(self.wait_with_output()?)
        }
        async fn stdout_to_stdio(&mut self) -> Option<Stdio> {
            self.stdout.take().map(Stdio::from)
        }

        fn kill(&mut self) -> AnyResult<()> {
            Ok(self.kill()?)
        }
    }

    #[async_trait]
    impl ChildUnify for SmolChild {
        async fn wait(mut self) -> AnyResult<ExitStatus> {
            #[cfg(target_family = "windows")]
            {
                return Ok(ExitStatus::from_raw(
                    self.status().await?.code().unwrap_or(1) as u32,
                ));
            }

            #[cfg(target_family = "unix")]
            return Ok(ExitStatus::from_raw(
                self.status().await?.code().unwrap_or(-1),
            ));
        }

        async fn wait_with_output(self) -> AnyResult<Output> {
            Ok(self.output().await?)
        }

        async fn stdout_to_stdio(&mut self) -> Option<Stdio> {
            if let Some(stdout) = self.stdout.take() {
                return stdout.into_stdio().await.ok();
            }
            None
        }

        fn kill(&mut self) -> AnyResult<()> {
            Ok(self.kill()?)
        }
    }

    #[async_trait]
    impl ChildUnify for TokioChild {
        async fn wait(self) -> AnyResult<ExitStatus> {
            Ok(self.wait().await?)
        }

        async fn wait_with_output(self) -> AnyResult<Output> {
            Ok(self.wait_with_output().await?)
        }

        async fn stdout_to_stdio(&mut self) -> Option<Stdio> {
            self.stdout.take().and_then(|s| s.try_into().ok())
        }

        // Attempts to force the child to exit, but does not wait for the request to take effect.
        // On Unix platforms, this is the equivalent to sending a SIGKILL.
        // Note that on Unix platforms it is possible for a zombie process to remain after a kill is sent;
        // to avoid this, the caller should ensure that either child.wait().await or child.try_wait() is invoked successfully.
        fn kill(&mut self) -> AnyResult<()> {
            Ok(self.start_kill()?)
        }
    }
    #[derive(Debug, Default)]
    /// Guarding of process handles.
    pub struct ChildGuard<Child: ChildUnify> {
        pub(crate) child: Option<Child>,
    }

    impl<Child: ChildUnify> ChildGuard<Child> {
        /// Build a `ChildGuard` with `Child`.
        pub fn new(child: Child) -> Self {
            let child = Some(child);
            Self { child }
        }

        /// Take inner `Child` from `ChildGuard`.
        pub fn take_inner(mut self) -> Option<Child> {
            self.child.take()
        }

        /// Await on `ChildGuard` and get `ExitStatus`.
        pub async fn wait(mut self) -> Result<ExitStatus, CommandChildError> {
            if let Some(child) = self.child.take() {
                return child
                    .wait()
                    .await
                    .map_err(|e| CommandChildError::DisCondition(e.to_string()));
            }

            Err(CommandChildError::DisCondition(
                "Without child for waiting.".to_string(),
            ))
        }

        /// Await on `ChildGuard` and get `Output`.
        pub async fn wait_with_output(mut self) -> Result<Output, CommandChildError> {
            if let Some(child) = self.child.take() {
                return child
                    .wait_with_output()
                    .await
                    .map_err(|e| CommandChildError::DisCondition(e.to_string()));
            }

            Err(CommandChildError::DisCondition(
                "Without child for waiting.".to_string(),
            ))
        }
    }

    impl<Child: ChildUnify> Drop for ChildGuard<Child> {
        fn drop(&mut self) {
            if let Some(child) = self.child.as_mut() {
                child
                    .kill()
                    .unwrap_or_else(|e| error!(" `ChildGuard` : {}", e));
            }
        }
    }

    impl<Child: ChildUnify> Deref for ChildGuard<Child> {
        type Target = Option<Child>;
        fn deref(&self) -> &Self::Target {
            &self.child
        }
    }

    impl<Child: ChildUnify> DerefMut for ChildGuard<Child> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.child
        }
    }

    //that code base on 'build-your-own-shell-rust'. Thanks you Josh Mcguigan.

    /// Generate a list of processes from a string of shell commands.
    //  There are a lot of system calls that happen when this method is called,
    //  the speed of execution depends on the parsing of the command and the speed of the process fork,
    //  after which it should be split into unblock().
    pub async fn parse_and_run<Child: ChildUnify, Command: CommandUnify<Child>>(
        input: &str,
    ) -> Result<ChildGuardList<Child>, CommandChildError> {
        // Check to see if process_linked_list is also automatically dropped out of scope
        // by ERROR's early return and an internal kill method is executed.

        let mut process_linked_list: ChildGuardList<Child> = LinkedList::new();
        // must be peekable so we know when we are on the last command
        let commands = input.trim().split(" | ").peekable();
        //if str has >> | > ,after spawn return.

        let mut _stdio: Stdio;

        for mut command in commands {
            let check_redirect_result = _has_redirect_file(command);
            if check_redirect_result.is_some() {
                command = _remove_angle_bracket_command(command)
                    .map_err(|e| CommandChildError::DisCondition(e.to_string()))?;
            }

            let mut parts = command.trim().split_whitespace();
            let command = parts
                .next()
                .ok_or_else(|| CommandChildError::DisCondition("Without next part".to_string()))?;
            let args = parts;

            // Standard input to the current process.
            // If previous_command is present, it inherits the standard output of the previous command.
            // If not, it inherits the current parent process.
            let previous_command = process_linked_list.back_mut();
            let mut stdin = Stdio::inherit();

            if let Some(previous_command_ref) = previous_command {
                let mut t = None;

                if let Some(child_mut) = previous_command_ref.child.as_mut() {
                    mem::swap(&mut child_mut.stdout_to_stdio().await, &mut t);
                }

                if let Some(child_stdio) = t {
                    stdin = child_stdio;
                }
            }

            let mut output = Command::new(command);
            output.args(args).stdin(stdin).stderr(Stdio::piped());

            let process: Child;
            let end_flag = if let Some(stdout_result) = check_redirect_result {
                let stdout =
                    stdout_result.map_err(|e| CommandChildError::DisCondition(e.to_string()))?;
                process = output
                    .stdout(stdout)
                    .spawn()
                    .map_err(|e| CommandChildError::DisCondition(e.to_string()))?;
                true
            } else {
                // if commands.peek().is_some() {
                //     // there is another command piped behind this one
                //     // prepare to send output to the next command
                let stdout = Stdio::piped();
                // } else {
                //     // there are no more commands piped behind this one
                //     // send output to shell stdout
                //     // TODO:It shouldn't be the standard output of the parent process in the context,
                //     // there should be a default file to record it.
                //     stdout = Stdio::inherit();
                // };

                process = output
                    .stdout(stdout)
                    .spawn()
                    .map_err(|e| CommandChildError::DisCondition(e.to_string()))?;
                false
            };

            process_linked_list.push_back(ChildGuard::<Child>::new(process));

            if end_flag {
                break;
            }
        }
        Ok(process_linked_list)
    }

    // I should give Option<Result<File>>
    //By Option(Some(Result<T>)), determine if there is an output stdio..
    //By Result<T>(OK(t)), determine if there is success open file.
    #[cfg(not(SPLIT_INCLUSIVE_COMPATIBLE))]
    fn _has_redirect_file(command: &str) -> Option<Result<File>> {
        let angle_bracket = if command.contains(">>") {
            ">>"
        } else if command.contains('>') {
            ">"
        } else {
            return None;
        };

        if let Some(filename) = command.trim().rsplit(angle_bracket).next() {
            Some(create_stdio_file(angle_bracket, filename))
        } else {
            None
        }
    }

    #[cfg(SPLIT_INCLUSIVE_COMPATIBLE)]
    fn _has_redirect_file(command: &str) -> Option<Result<File, AnyhowError>> {
        let angle_bracket = if command.contains(">>") {
            ">>"
        } else if command.contains('>') {
            ">"
        } else {
            return None;
        };

        let mut sub_command_inner = command.trim().split_inclusive(angle_bracket).rev();

        sub_command_inner
            .next()
            .map(|filename| create_stdio_file(angle_bracket, filename))
    }

    //After confirming that there is a redirect file, parse the command before the command '>'.
    fn _remove_angle_bracket_command(command: &str) -> Result<&str, AnyhowError> {
        let mut sub_command_inner = command.trim().split('>');
        sub_command_inner
            .next()
            .ok_or_else(|| anyhow!("can't parse ...."))
    }

    fn create_stdio_file(angle_bracket: &str, filename: &str) -> Result<File, AnyhowError> {
        let mut file_tmp = OpenOptions::new();
        file_tmp.write(true).create(true);

        if angle_bracket == ">>" {
            file_tmp.append(true);
        }

        //TODO:I need record that open file error because filename has a whitespace i don't trim.
        let os_filename = Path::new(filename.trim()).as_os_str();

        let stdio_file = file_tmp.open(os_filename)?;
        Ok(stdio_file)
    }
}
